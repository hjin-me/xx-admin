use crate::backend::xxscore::fetcher::browse_xx_admin;
use crate::state::{State, StateChange};
use anyhow::{anyhow, Result};
use std::ops::{Add, Deref};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};

#[derive(Clone)]
pub struct XxAdmin {
    available_before: chrono::DateTime<chrono::Local>,
    state: Arc<RwLock<State>>,
    cancel: CancellationToken,
}

impl XxAdmin {
    pub fn new(xx_org_gray_id: &str, proxy_server: Option<String>) -> Result<Self> {
        let cancel_token = CancellationToken::new();
        let (tx, rx) = std::sync::mpsc::channel::<StateChange>();

        let cloned_cancel_token = cancel_token.clone();
        let cloned_xx_org_gray_id = xx_org_gray_id.to_string();
        thread::spawn(move || {
            let run = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(anyhow!("XxState 启动后台任务失败: {}", e)),
            };
            match run.block_on(async {
                let proxy_server = proxy_server.clone();
                tokio::select! {
                    _ = cloned_cancel_token.cancelled() => {
                        info!("admin 后台任务被取消");
                        return Err(anyhow!("进程退出，任务正常取消"))
                    }
                    r = browse_xx_admin(tx.clone(), &cloned_xx_org_gray_id, &proxy_server) => {
                        trace!("后台任务好像执行完了");
                        r
                    }
                }
            }) {
                Ok(r) => {
                    info!("admin 后台任务完成");
                    tx.send(StateChange::Complete(r))?
                }
                Err(e) => {
                    error!("admin 后台任务失败: {}", e);
                    tx.send(StateChange::BrowserClosed(e))?
                }
            };
            Ok(())
        });
        let state = Arc::new(RwLock::new(State::Prepare));
        let state_self = state.clone();
        thread::spawn(move || {
            for x in rx.iter() {
                match x {
                    StateChange::BrowserClosed(e) => {
                        error!("浏览器崩溃了: {}", e);
                        let mut s = state.write().unwrap();
                        *s = State::Broken(e.to_string());
                        return;
                    }
                    StateChange::Init => {
                        trace!("浏览器开始初始化");
                        let mut s = state.write().unwrap();
                        *s = State::Init;
                    }
                    StateChange::Ready => {
                        trace!("ready");
                        let mut s = state.write().unwrap();
                        *s = State::Ready;
                    }
                    StateChange::WaitingLogin(ticket) => {
                        info!("等待登陆: {}", ticket);
                        let mut s = state.write().unwrap();
                        *s = State::WaitingLogin((
                            ticket,
                            chrono::Local::now()
                                .add(Duration::from_secs(150))
                                .timestamp(),
                        ))
                    }
                    StateChange::LoggedIn => {
                        info!("登陆成功");
                        let mut s = state.write().unwrap();
                        *s = State::Logged;
                    }
                    StateChange::Complete(ms) => {
                        info!("学习完成: {:?}", ms);
                        let mut s = state.write().unwrap();
                        *s = State::Complete(ms);
                        return;
                    }
                }
            }
        });

        Ok(Self {
            available_before: chrono::Local::now().add(Duration::from_secs(200)),
            state: state_self,
            cancel: cancel_token.clone(),
        })
    }

    pub fn get_state(&self) -> State {
        // trace!("get_state");
        match self.state.read() {
            Ok(s) => (*s.deref()).clone(),
            Err(_) => State::Broken("读取状态失败".to_string()),
        }
    }

    pub fn ping(&self) -> bool {
        trace!("ping");
        self.available_before > chrono::Local::now()
            && match self.get_state() {
                State::Init => true,
                State::Prepare => true,
                State::WaitingLogin((_, ts)) => ts > chrono::Local::now().timestamp(),
                _ => false,
            }
    }
}

impl Drop for XxAdmin {
    fn drop(&mut self) {
        debug!("drop XxAdmin");
        self.cancel.cancel();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_xx_admin() -> Result<()> {
        tracing_subscriber::fmt::init();
        info!("开始了");
        let xa = XxAdmin::new("zW2GdDXrYrFXV3GOz5j6eg==", None)?;
        info!("start");

        loop {
            info!("state is {:?}", xa.get_state());
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}
