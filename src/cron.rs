use crate::config::Config;
use crate::xxscore::fetcher::FetcherImpl;
use crate::xxscore::{daily_score, get_yesterday};
use anyhow::Result;
use chrono::{Local, Timelike};
use reqwest::ClientBuilder;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, trace, warn};

pub async fn start_daily_score(p: &Config) -> Result<()> {
    let p = p.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        let xx_fetcher = FetcherImpl::new(
            &p.admin_user,
            &p.xx_org_gray_id,
            &p.wechat_proxy,
            p.proxy_server.clone(),
        );

        let http_client = {
            let b = ClientBuilder::default();
            match p.proxy_server.as_ref() {
                Some(s) => b
                    .proxy(reqwest::Proxy::all(s.to_owned()).expect("解析 proxy 格式失败"))
                    .build()
                    .expect("初始化 http client 失败"),
                None => b.no_proxy().build().expect("初始化 http client 失败"),
            }
        };

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
                    &http_client,
                    &yesterday,
                    &xx_fetcher,
                    p.notice_bot.iter().map(|s| s.as_str()).collect(),
                    p.org_id,
                    &p.admin_user,
                    &p.wechat_proxy,
                ),
            )
            .await
            {
                Ok(r) => match r {
                    Ok(_) => {
                        info!("{} 学习积分统计成功", yesterday);
                    }
                    Err(e) => {
                        warn!("Error: {:?}", e);
                    }
                },
                Err(e) => {
                    warn!("Error: {:?}", e);
                }
            };
        }
    })
    .await?;
    Ok(())
}
