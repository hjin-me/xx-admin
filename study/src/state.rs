use crate::{XxManager, XxManagerPool};
use anyhow::{anyhow, Error, Result};
use bb8::PooledConnection;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

enum StateChange {
    BrowserClosed(Error),
    Ready,
    WaitingLogin(String),
    LoggedIn(String),
    StartLearn,
    Complete(i64),
}
#[derive(Default, Debug)]
struct State {
    pub broken: bool,
    pub login_ticket: String,
    pub nick_name: String,
    pub score: Option<i64>,
}

#[derive(Clone)]
pub struct XxState {
    state: Arc<RwLock<State>>,
}

impl XxState {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::default())),
        }
    }

    pub fn serve(&self, pool: XxManagerPool) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel::<StateChange>();
        thread::spawn(move || {
            let run = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(anyhow!("XxState 启动后台任务失败: {}", e)),
            };
            match run.block_on(async {
                info!("get pool");
                let mut conn = match pool.get().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        match e {
                            bb8::RunError::User(e) => {
                                error!("获取连接失败: {}", e);
                            }
                            bb8::RunError::TimedOut => {
                                error!("获取连接超时");
                            }
                        }
                        return Err(anyhow!("获取连接池失败了"));
                    }
                };
                tx.send(StateChange::Ready)?;
                let ticket = conn.get_ticket();
                tx.send(StateChange::WaitingLogin(ticket))?;
                info!("got");
                waiting_login(&mut conn, Duration::from_secs(120)).await?;
                let nick_name = conn.get_user_info()?;
                tx.send(StateChange::LoggedIn(nick_name))?;

                let news_list = vec!["https://www.xuexi.cn/lgpage/detail/index.html?id=1675585234174641917&item_id=1675585234174641917".to_string()];
                let video_list :Vec<String>= vec![];
                tx.send(StateChange::StartLearn)?;
                conn.try_study(&news_list, &video_list)?;
                let n = conn.get_today_score()?;
                Ok(n)
            }) {
                Ok(n) => {
                    tx.send(StateChange::Complete(n))?;
                }
                Err(e) => {
                    error!("XxState 后台任务失败: {}", e);
                    tx.send(StateChange::BrowserClosed(e))?
                }
            };
            Ok(())
        });
        let state = self.state.clone();
        thread::spawn(move || {
            for x in rx.iter() {
                match x {
                    StateChange::BrowserClosed(e) => {
                        error!("浏览器崩溃了: {}", e);
                        state.write().unwrap().broken = true;
                    }
                    StateChange::Ready => {
                        info!("ready");
                    }
                    StateChange::WaitingLogin(ticket) => {
                        let mut s =
                            "https://techxuexi.js.org/jump/techxuexi-20211023.html?".to_string();
                        s.extend(form_urlencoded::byte_serialize(ticket.as_bytes()));
                        info!("等待登陆: {}", s);
                        state.write().unwrap().login_ticket = ticket;
                    }
                    StateChange::LoggedIn(nick_name) => {
                        info!("登陆成功: {}", nick_name);
                        state.write().unwrap().nick_name = nick_name;
                    }
                    StateChange::StartLearn => {
                        info!("开始学习");
                    }
                    StateChange::Complete(i) => {
                        info!("学习完成: {}", i);
                        state.write().unwrap().score = Some(i);
                    }
                }
            }
        });
        Ok(())
    }
}

async fn waiting_login(
    conn: &mut PooledConnection<'_, XxManager>,
    timeout: Duration,
) -> Result<()> {
    let check = async {
        loop {
            match conn.check_login() {
                Ok(b) => {
                    if b {
                        break;
                    } else {
                        info!("还没登陆");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
                Err(e) => {
                    error!("判断登陆状态失败: {}", e);
                }
            }
        }
    };
    tokio::select! {
        _ = check => {
            return Ok(())
        },
        _ = tokio::time::sleep(timeout) => {
            warn!("等待登陆超时");
            return Err(anyhow!("等待登陆超时"))
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn test_state() -> Result<()> {
        tracing_subscriber::fmt::init();
        let manager = XxManager::new();
        let pool = bb8::Pool::builder()
            .max_size(2)
            .min_idle(Some(1))
            .idle_timeout(Some(Duration::from_secs(170)))
            // .connection_timeout(std::time::Duration::from_secs(30))
            .build(manager)
            .await
            .unwrap();
        let state = XxState::new();
        state.serve(pool)?;
        loop {
            {
                let s = state.state.read().unwrap();
                info!("读取状态数据 {:?}", s);
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}
