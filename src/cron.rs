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
            if d.minute() == 30 {
                info!("继续等待任务开始执行");
            }

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
    let mut handles = vec![];

    for c in p.study_schedule {
        let corp_id = p.corp_id.clone();
        let corp_secret = p.corp_secret.clone();
        let proxy_server = p.proxy_server.clone();
        handles.push(tokio::spawn(async move {
            info!("学习开始");
            let mut ticker = interval(Duration::from_secs(60));

            let mp = wx::MP::new(&corp_id, &corp_secret, p.agent_id);

            loop {
                ticker.tick().await;
                let d = Local::now();
                if d.minute() == 30 {
                    info!("继续等待任务开始执行");
                }

                if d.hour() != c.hour || d.minute() != c.minute {
                    trace!("not time yet");
                    continue;
                }

                match tokio::time::timeout(
                    Duration::from_secs(2 * 60 * 60),
                    browse_xx(&mp, &c.target, &proxy_server),
                )
                .await
                {
                    Ok(r) => match r {
                        Ok(_) => {
                            info!("今天的学习强国就逛到这里了[{}]", &c.target);
                        }
                        Err(e) => {
                            warn!("学习任务执行失败: [{}] {:?}", &c.target, e);
                        }
                    },
                    Err(e) => {
                        warn!("学习任务超时: [{}] {:?}", &c.target, e);
                    }
                };
            }
        }))
    }
    for handle in handles {
        handle.await?;
    }
    Ok(())
}
