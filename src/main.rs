mod config;
mod cron;
mod wx;
mod xxscore;

use crate::config::Config;
use crate::cron::start_daily_score;
use anyhow::Result;
use clap::Parser;
use tokio::fs;
use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of times to greet
    #[arg(short, long, default_value = "./config.toml")]
    config: String,
    #[arg(short, long, default_value = "info")]
    log: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .json()
        .with_max_level(args.log.parse::<Level>().unwrap_or(Level::INFO))
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    // get pwd
    let pwd = std::env::current_dir().unwrap();
    info!(conf_path = &args.config, cwd = ?pwd, "Starting up",);
    info!("Version: {}", env!("COMMIT_ID"));
    let contents = fs::read_to_string(&args.config).await?;
    let serv_conf: Config = toml::from_str(contents.as_str())?;

    start_daily_score(&serv_conf).await?;

    Ok(())
}
