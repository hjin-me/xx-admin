use crate::config::{AdminConfig, StudyConfig};
use crate::xxscore::fetcher::FetcherImpl;
use crate::xxscore::{daily_score, get_yesterday};
use anyhow::Result;
use chrono::{Local, Timelike};
use std::time::Duration;
use study::browse_xx;
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

pub async fn start_daily_study(conf_path: &str) -> Result<()> {
    let contents = fs::read_to_string(conf_path).await?;
    let p: StudyConfig = toml::from_str(contents.as_str())?;
    info!("学习任务定时任务已启动");
    let mut ticker = interval(Duration::from_secs(60));

    let mp = wx::MP::new(&p.corp_id, &p.corp_secret, p.agent_id);

    loop {
        ticker.tick().await;
        trace!("每分钟定时任务检查");
        let d = Local::now();
        let tasks = {
            let contents = fs::read_to_string(conf_path).await?;
            let p: StudyConfig = toml::from_str(contents.as_str())?;
            p.study_schedule
        };

        for x in tasks {
            if x.hour != d.hour() || x.minute != d.minute() {
                continue;
            }
            let proxy_server = p.proxy_server.clone();
            let mp = mp.clone();
            tokio::spawn(async move {
                info!(
                    user = &x.target,
                    hour = x.hour,
                    minute = x.minute,
                    "时间到了，开始学习任务"
                );
                match tokio::time::timeout(
                    Duration::from_secs(2 * 60 * 60),
                    browse_xx(&mp, &x.target, &proxy_server),
                )
                .await
                {
                    Ok(r) => match r {
                        Ok(_) => {
                            info!(user = x.target, "今天的学习强国就逛到这里了");
                        }
                        Err(e) => {
                            warn!(user = x.target, "学习任务执行失败: {}", e);
                        }
                    },
                    Err(e) => {
                        warn!(user = x.target, "学习任务超时: {}", e);
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_study() -> Result<()> {
        let _g = crate::otel::init_tracing_subscriber("xx-debug", "");
        let contents = include_str!("../config.toml");
        let p: StudyConfig = toml::from_str(contents)?;
        // start_daily_study_schedule(&p.corp_id, &p.corp_secret, p.agent_id, 16, 42, "", &None).await;
        Ok(())
    }
}
