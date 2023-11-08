mod config;
mod cron;
mod health;
mod serv;
mod xxscore;

use crate::cron::{start_daily_score, start_daily_study};
use anyhow::Result;
use clap::Parser;
use tokio::signal;
use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of times to greet
    #[arg(short, long, default_value = "./config.toml")]
    config: String,
    #[arg(short, long, default_value = "info")]
    log: String,
    #[arg(long, default_value = "admin")]
    cmd: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .json()
        .with_max_level(args.log.parse::<Level>().unwrap_or(Level::INFO))
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let pwd = std::env::current_dir().unwrap();
    info!(conf_path = &args.config, cwd = ?pwd, "Starting up",);
    info!("Version: {}", env!("COMMIT_ID"));
    match args.cmd.as_str() {
        "admin" => tokio::select! {
            r = start_daily_score(&args.config) => {
                r?
            },
            _ = signal::ctrl_c() => {
                info!("收到退出命令");
            },
        },
        _ => tokio::select! {
            r = start_daily_study(&args.config) => {
                r?
            },
            _ = signal::ctrl_c() => {
                info!("收到退出命令");
            },
        },
    };

    Ok(())
}
