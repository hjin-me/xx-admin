use crate::utils::UserValidator;
use crate::{new_xx_task_bg, State, StateChange};
use anyhow::{anyhow, Result};
use std::ops::{Add, Deref};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};

#[derive(Clone)]
pub struct Xx {
    available_before: chrono::DateTime<chrono::Local>,
    state: Arc<RwLock<State>>,
    cancel: CancellationToken,
}

impl Xx {
    pub fn new<T: UserValidator + Send + Sync + Clone + 'static>(
        validator: T,
        proxy_server: Option<String>,
    ) -> Result<Self> {
        let cancel_token = CancellationToken::new();
        let (tx, rx) = std::sync::mpsc::channel::<StateChange>();

        let cloned_cancel_token = cancel_token.clone();
        thread::spawn(move || {
            let run = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(anyhow!("XxState 启动后台任务失败: {}", e)),
            };
            match run.block_on(async {
                tokio::select! {
                    _ = cloned_cancel_token.cancelled() => {
                        info!("study 后台任务被取消");
                        return Err(anyhow!("进程退出，任务正常取消"))
                    }
                    r = new_xx_task_bg(tx.clone(), validator, proxy_server) => {
                        trace!("后台任务好像执行完了");
                        r
                    }
                }
            }) {
                Ok(_) => {
                    info!("study 后台任务完成");
                }
                Err(e) => {
                    error!("study 后台任务失败: {}", e);
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
                        let mut s =
                            "https://techxuexi.js.org/jump/techxuexi-20211023.html?".to_string();
                        s.extend(form_urlencoded::byte_serialize(ticket.as_bytes()));
                        info!("等待登陆: {}", s);
                        let mut s = state.write().unwrap();
                        *s = State::WaitingLogin((
                            ticket,
                            chrono::Local::now()
                                .add(Duration::from_secs(150))
                                .timestamp(),
                        ))
                    }
                    StateChange::LoggedIn(user_info) => {
                        info!("登陆成功: {}", user_info.nick);
                        let mut s = state.write().unwrap();
                        *s = State::Logged(user_info.nick);
                    }
                    StateChange::StartLearn => {
                        info!("开始学习");
                    }
                    StateChange::LearnLog((nick_name, logs)) => {
                        info!("学习进行中: {} {:?}", nick_name, logs);
                        let mut s = state.write().unwrap();
                        *s = State::Learning((nick_name, logs));
                    }
                    StateChange::Complete(r) => {
                        info!("学习完成: {} {}", r.0, r.1);
                        let mut s = state.write().unwrap();
                        *s = State::Complete(r);
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

    pub fn is_valid(&self) -> bool {
        trace!("is_valid");
        self.available_before > chrono::Local::now()
            && match self.get_state() {
                State::WaitingLogin((_, ts)) => ts > chrono::Local::now().timestamp(),
                _ => false,
            }
    }
}

impl Drop for Xx {
    fn drop(&mut self) {
        debug!("drop Xx");
        self.cancel.cancel();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use async_trait::async_trait;
    use sysinfo::{ProcessExt, System, SystemExt};

    #[derive(Clone)]
    struct MockUV {}

    #[async_trait]
    impl UserValidator for MockUV {
        async fn validate(&self, _: i64) -> Result<bool> {
            Ok(true)
        }
    }
    #[test]
    fn test_chrome_leak() -> Result<()> {
        tracing_subscriber::fmt::init();
        let mut system = System::new_all();

        // 更新所有进程信息
        system.refresh_processes();

        println!("启动前 Chrome 进程列表");
        println!("=======");
        // 获取所有进程列表
        let before_pids_count = system
            .processes()
            .iter()
            .filter(|(pid, process)| {
                if process.name().contains("hrome") {
                    println!("pid: {}, name: {}", pid, process.name());
                    !process.name().contains("crashpad")
                } else {
                    false
                }
            })
            .count();

        let xx = Xx::new(MockUV {}, None)?;
        loop {
            if xx.is_valid() {
                break;
            }
            thread::sleep(Duration::from_secs(1))
        }
        drop(xx);
        thread::sleep(Duration::from_secs(5));
        println!("运行后的 Chrome 进程列表");
        println!("===================");
        // 更新所有进程信息
        system.refresh_processes();

        // 获取所有进程列表
        let after_pids_count = system
            .processes()
            .iter()
            .filter(|(pid, process)| {
                if process.name().contains("hrome") {
                    println!("pid: {}, name: {}", pid, process.name());
                    !process.name().contains("crashpad")
                } else {
                    false
                }
            })
            .count();

        assert_eq!(after_pids_count, before_pids_count);

        Ok(())
    }
}
