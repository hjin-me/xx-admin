pub mod config;
pub mod cron;
mod push_notice;
mod session;
mod xxscore;
pub mod api;

use crate::backend::config::AdminConfig;
use anyhow::Result;
use cron::start_daily_notice;
pub use session::StateSession;
use std::fs;
use tokio::signal;
use tracing::info;
use wx::MP;

pub async fn serve(config: &str) -> Result<()> {
    tokio::select! {
        r = start_daily_notice(config) => {
            r?
        },
        _ = signal::ctrl_c() => {
            info!("收到退出命令");
        },
    }
    Ok(())
}
