mod config;
mod cron;
mod health;
mod serv;
mod xxscore;

use crate::cron::start_daily_score;
use anyhow::Result;
use clap::Parser;
use std::env;
use tokio::signal;
use tracing::{debug, info};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of times to greet
    #[arg(short, long, default_value = "./config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let _g = infra::otel::init_tracing_subscriber("admin");

    let pwd = env::current_dir().unwrap();
    debug!(conf_path = &args.config, cwd = ?pwd, "Starting up",);
    info!("Version: {}", env!("COMMIT_ID"));
    debug!("RUST_LOG: {:?}", env::var_os("RUST_LOG"));
    tokio::select! {
        r = start_daily_score(&args.config) => {
            r?
        },
        _ = signal::ctrl_c() => {
            info!("收到退出命令");
        },
    }

    Ok(())
}
