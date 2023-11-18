#[cfg(feature = "server")]
use crate::utils::{get_news_list, get_video_list};
#[cfg(feature = "server")]
use crate::{XxManager, XxManagerPool};
#[cfg(feature = "server")]
use anyhow::{anyhow, Error, Result};
#[cfg(feature = "server")]
use bb8::PooledConnection;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use std::ops::Deref;
#[cfg(feature = "server")]
use std::sync::{Arc, RwLock};
#[cfg(feature = "server")]
use std::thread;
#[cfg(feature = "server")]
use std::time::Duration;
// use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "server")]
enum StateChange {
    BrowserClosed(Error),
    Ready,
    WaitingLogin(String),
    LoggedIn(String),
    StartLearn,
    Complete((String, i64)),
}
#[cfg(feature = "hydrate")]
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Ticket {
    pub ticket: String,
}
#[cfg(feature = "hydrate")]
#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum State {
    Broken(String),
    Prepare,
    Init,
    Ready,
    WaitingLogin(Ticket),
    Logged(String),
    Complete((String, i64)),
}

#[cfg(feature = "server")]
#[derive(Clone)]
pub struct XxState {
    state: Arc<RwLock<State>>,
}

#[cfg(feature = "server")]
impl XxState {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(State::Prepare)),
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
                trace!("got");
                waiting_login(&mut conn, Duration::from_secs(120)).await?;
                let nick_name = conn.get_user_info()?;
                tx.send(StateChange::LoggedIn(nick_name.clone()))?;

                let news_list = get_news_list().await?;
                let video_list = get_video_list().await?;

                tx.send(StateChange::StartLearn)?;
                conn.try_study(&news_list, &video_list)?;
                let n = conn.get_today_score()?;
                Ok((nick_name, n))
            }) {
                Ok(r) => {
                    tx.send(StateChange::Complete(r))?;
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
                        let mut s = state.write().unwrap();
                        *s = State::Broken(e.to_string());
                    }
                    StateChange::Ready => {
                        trace!("ready");
                        let mut s = state.write().unwrap();
                        *s = State::Ready;
                    }
                    StateChange::WaitingLogin(ticket) => {
                        let mut s =
                            "https://techxuexi.js.org/jump/techxuexi-20211023.html?".to_string();
                        s.extend(form_urlencoded::byte_serialize(ticket.as_bytes()));
                        info!("等待登陆: {}", s);
                        let mut s = state.write().unwrap();
                        *s = State::WaitingLogin(Ticket { ticket })
                    }
                    StateChange::LoggedIn(nick_name) => {
                        info!("登陆成功: {}", nick_name);
                        let mut s = state.write().unwrap();
                        *s = State::Logged(nick_name);
                    }
                    StateChange::StartLearn => {
                        info!("开始学习");
                    }
                    StateChange::Complete(r) => {
                        info!("学习完成: {} {}", r.0, r.1);
                        let mut s = state.write().unwrap();
                        *s = State::Complete(r);
                    }
                }
            }
        });
        {
            let mut s = self.state.write().unwrap();
            *s = State::Init;
        }
        Ok(())
    }

    pub fn get_state(&self) -> State {
        let s = self.state.read().unwrap();
        (*s.deref()).clone()
    }

    pub fn get_ticket(&self) -> Result<String> {
        let s = self.get_state();
        match s {
            State::WaitingLogin(t) => Ok(t.ticket.clone()),
            _ => Err(anyhow!("还没有获取到 ticket")),
        }
    }

    pub fn get_nick_name(&self) -> Result<String> {
        let s = self.get_state();
        match s {
            State::WaitingLogin(_) => Err(anyhow!("还没有登陆")),
            State::Logged(n) => Ok(n.clone()),
            _ => Err(anyhow!("还没有获取到 nick_name")),
        }
    }
    pub fn get_score(&self) -> Result<i64> {
        let s = self.get_state();
        match s {
            State::WaitingLogin(_) => Err(anyhow!("还没有登陆")),
            State::Logged(_) => Err(anyhow!("还没有开始学习")),
            State::Complete((_, t)) => Ok(t),
            _ => Err(anyhow!("还没有获取到 score")),
        }
    }
}

#[cfg(feature = "server")]
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
                        debug!("还没登陆");
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

#[cfg(all(feature = "server", test))]
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
                let s = state.get_state();
                info!("读取状态数据 {:?}", s);
                match s {
                    State::Broken(e) => {
                        error!("浏览器崩溃了: {}", e);
                        break;
                    }
                    State::Complete((n, t)) => {
                        info!("学习完成: {} {}", n, t);
                        break;
                    }
                    _ => {}
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
        Ok(())
    }
}

// #[derive(Deserialize, Serialize)]
// pub struct Ticket {
//     pub url: String,
//     pub ticket: String,
//     pub data_uri: String, // data:image/png;base64,i
// }
// impl Ticket {
//     pub fn new(u: &str) -> Self {
//         let mut ticket = "".to_string();
//         ticket.extend(form_urlencoded::byte_serialize(u.as_bytes()));
//         Self {
//             url: u.to_string(),
//             ticket,
//             data_uri: "data:image/png;base64,".to_string(),
//         }
//     }
// }
//
