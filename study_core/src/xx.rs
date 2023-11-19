use crate::{new_xx_task_bg, State, StateChange};
use anyhow::{anyhow, Result};
use std::ops::{Add, Deref};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tracing::{error, info, trace};

#[derive(Clone)]
pub struct Xx {
    available_before: chrono::DateTime<chrono::Local>,
    state: Arc<RwLock<State>>,
}

impl Xx {
    pub fn new() -> Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel::<StateChange>();
        thread::spawn(move || {
            let run = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(anyhow!("XxState 启动后台任务失败: {}", e)),
            };
            match run.block_on(async {
                new_xx_task_bg(tx.clone()).await?;
                Ok(())
            }) {
                Ok(_) => {
                    info!("XxState 后台任务完成");
                }
                Err(e) => {
                    error!("XxState 后台任务失败: {}", e);
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
                        trace!("init");
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
                    StateChange::LoggedIn(nick_name) => {
                        info!("登陆成功: {}", nick_name);
                        let mut s = state.write().unwrap();
                        *s = State::Logged(nick_name);
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
        })
    }

    pub fn get_state(&self) -> State {
        trace!("get_state");
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

#[cfg(test)]
mod test {

}