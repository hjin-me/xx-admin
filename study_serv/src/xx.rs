use anyhow::{anyhow, Result};
use dioxus_fullstack::prelude::extract;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use study::bb8::Pool;
use study::{bb8, StateSession, XxManager};
use study_core::State;
use tokio::time;
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StudyConfig {
    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: i64,
    pub app_caller: String,
}

#[instrument]
pub async fn try_get_ticket(s_id: u64) -> Result<String> {
    use crate::backend::user_validator::WBList;
    use axum::Extension;
    use study::StateSession;
    let Extension(ss): Extension<StateSession<WBList>> = extract().await?;

    let state = ss.get(s_id).ok_or(anyhow!("没有找到状态数据"))?;

    for i in 1..11 {
        match state.get_ticket() {
            Ok(s) => {
                return Ok(s);
            }
            Err(e) => {
                warn!("获取 ticket 失败[第{}次]: {}", i, e);
            }
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(anyhow!("获取 ticket 失败"))
}
#[instrument]
pub async fn try_get_current_user(s_id: u64) -> Result<String> {
    use crate::backend::user_validator::WBList;
    use axum::Extension;
    use study::StateSession;
    let Extension(ss): Extension<StateSession<WBList>> = extract().await?;

    let state = ss.get(s_id).ok_or(anyhow!("没有找到状态数据"))?;

    for i in 1..11 {
        match state.get_nick_name() {
            Ok(s) => {
                return Ok(s);
            }
            Err(e) => {
                warn!("获取用户名失败[第{}次]: {}", i, e);
            }
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(anyhow!("获取用户名失败"))
}

#[instrument(level = "trace")]
pub async fn try_get_state(s_id: u64) -> Result<State> {
    use crate::backend::user_validator::WBList;
    use axum::Extension;
    use study::StateSession;
    let Extension(ss): Extension<StateSession<WBList>> = extract().await?;

    let state = ss.get(s_id).ok_or(anyhow!("没有找到状态数据"))?;

    Ok(state.get_state().0)
}

#[instrument(level = "trace")]
pub async fn start_new_task() -> Result<u64> {
    use crate::backend::user_validator::WBList;
    use axum::Extension;
    use study::StateSession;
    let Extension(ss): Extension<StateSession<WBList>> = extract().await?;
    let s_id = ss.new_state()?;
    Ok(s_id)
}
#[instrument]
pub async fn get_today_report() -> Result<Vec<State>> {
    use crate::backend::user_validator::WBList;
    use axum::Extension;
    use chrono::Local;
    use study::StateSession;
    let Extension(ss): Extension<StateSession<WBList>> = extract().await?;

    let states = ss.get_history(Local::now());

    Ok(states)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_study() {
        tracing_subscriber::fmt::init();
        info!("{:?}", tokio::runtime::Handle::current().runtime_flavor());
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            info!("block_on");
            info!("{:?}", tokio::runtime::Handle::current().runtime_flavor());
            tokio::spawn(async {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        info!("sleep 2 end");
                    });
                });
            });
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            info!("sleep 10 end");
        });
        info!("end");
    }
}
