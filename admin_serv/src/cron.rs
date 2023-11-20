use crate::config::AdminConfig;
use crate::xxscore::fetcher::FetcherImpl;
use crate::xxscore::{daily_score, get_yesterday};
use anyhow::Result;
use chrono::{Local, Timelike};
use std::time::Duration;
use tokio::fs;
use tokio::time::interval;
use tracing::{info, trace, warn};

pub async fn start_daily_score(conf_path: &str) -> Result<()> {
    let contents = fs::read_to_string(conf_path).await?;
    let p: AdminConfig = toml::from_str(contents.as_str())?;
    tokio::spawn(async move {
        info!("学习管理员开始");
        let mut ticker = interval(Duration::from_secs(60));

        let mp = wx::MP::new(&p.corp_id, &p.corp_secret, p.agent_id);

        let xx_fetcher = FetcherImpl::new(
            &p.admin_user,
            &p.xx_org_gray_id,
            &mp,
            p.proxy_server.clone(),
            &p.app_caller,
        );

        loop {
            ticker.tick().await;
            let d = Local::now();

            if d.hour() != p.exec_hour || d.minute() != p.exec_minute {
                trace!("not time yet");
                continue;
            }
            info!("开始执行积分统计任务，会启动 Chrome 进行数据抓取");
            let yesterday = get_yesterday();

            match tokio::time::timeout(
                Duration::from_secs(2 * 60 * 60),
                daily_score(
                    &yesterday,
                    &xx_fetcher,
                    p.notice_bot.iter().map(|s| s.as_str()).collect(),
                    p.org_id,
                    &p.admin_user,
                    &mp,
                ),
            )
            .await
            {
                Ok(r) => match r {
                    Ok(_) => {
                        info!("{} 学习积分统计成功", yesterday);
                    }
                    Err(e) => {
                        warn!("管理员任务执行异常: {:?}", e);
                    }
                },
                Err(e) => {
                    warn!("管理员任务执行超时: {:?}", e);
                }
            };
        }
    })
    .await?;
    Ok(())
}
