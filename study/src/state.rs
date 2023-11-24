#[cfg(feature = "server")]
use crate::XxManagerPool;
#[cfg(feature = "server")]
use anyhow::{anyhow, Result};
#[cfg(feature = "server")]
use std::ops::Deref;
#[cfg(feature = "server")]
use std::sync::{Arc, RwLock};
#[cfg(feature = "server")]
use std::thread;
#[cfg(feature = "server")]
use std::time::Duration;
use study_core::utils::UserValidator;
use study_core::State;
use tokio::time::sleep;
#[cfg(feature = "server")]
use tokio_util::sync::CancellationToken;
#[cfg(feature = "server")]
use tracing::{error, info};
use tracing::instrument;

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

    #[instrument(skip_all, level = "trace")]
    pub fn serve<T: UserValidator + Send + Sync + Clone + 'static>(
        &self,
        pool: XxManagerPool<T>,
    ) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel::<State>();
        let cancel_token = CancellationToken::new();
        let cloned_cancel_token = cancel_token.clone();
        thread::spawn(move || {
            let run = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(anyhow!("XxState 启动后台任务失败: {}", e)),
            };
            run.spawn(async move {
                sleep(Duration::from_secs(5 * 60)).await;
                cloned_cancel_token.cancel();
            });
            match run.block_on(async {
                info!("get pool");
                let conn = match pool.get().await {
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
                loop {
                    let state = conn.get_state();
                    tx.send(state.clone())?;
                    match state.clone() {
                        State::Complete(_) => return Ok(()),
                        State::Broken(e) => return Err(anyhow!(e)),
                        State::WaitingLogin(_) => {
                            if cancel_token.is_cancelled() {
                                tx.send(State::Broken("等了5分钟你都没登陆".to_string()))?;
                                return Err(anyhow!("5分钟都没有主动登陆学习，任务取消了"));
                            }
                        }
                        _ => {}
                    };
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }) {
                Ok(_) => {}
                Err(e) => {
                    error!("XxState 后台任务失败: {}", e);
                    tx.send(State::Broken(e.to_string()))?
                }
            };
            Ok(())
        });
        let state = self.state.clone();
        thread::spawn(move || {
            for x in rx.iter() {
                let mut s = state.write().unwrap();
                *s = x.clone();
            }
        });

        Ok(())
    }

    #[instrument(skip_all, level = "trace")]
    pub fn get_state(&self) -> State {
        let s = self.state.read().unwrap();
        (*s.deref()).clone()
    }

    #[instrument(skip_all, level = "trace")]
    pub fn get_ticket(&self) -> Result<String> {
        let s = self.get_state();
        match s {
            State::WaitingLogin((t, ts)) => {
                if ts < chrono::Local::now().timestamp() {
                    return Err(anyhow!("ticket 已经过期"));
                }
                Ok(t.clone())
            }
            _ => Err(anyhow!("还没有获取到 ticket")),
        }
    }

    #[instrument(skip_all, level = "trace")]
    pub fn get_nick_name(&self) -> Result<String> {
        let s = self.get_state();
        match s {
            State::WaitingLogin(_) => Err(anyhow!("还没有登陆")),
            State::Logged(n) => Ok(n.clone()),
            _ => Err(anyhow!("还没有获取到 nick_name")),
        }
    }
    #[instrument(skip_all, level = "trace")]
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

#[cfg(all(feature = "server", test))]
mod test {
    use super::*;
    use crate::XxManager;
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
