use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StudyConfig {
    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: i64,
    pub app_caller: String,
}
pub async fn run() -> Result<()> {
    info!("{:?}", tokio::runtime::Handle::current().runtime_flavor());
    let contents = include_str!("../../config.toml");
    let p: StudyConfig = toml::from_str(contents)?;
    tokio::runtime::Runtime::new()?
        .spawn(async move {
            start_daily_study_schedule(
                &p.corp_id,
                &p.corp_secret,
                p.agent_id,
                "",
                &None,
                &p.app_caller,
            )
            .await;
        })
        .await?;
    Ok(())
}
#[instrument(skip_all, fields(user = %target))]
async fn start_daily_study_schedule(
    corp_id: &str,
    corp_secret: &str,
    agent_id: i64,
    target: &str,
    proxy_server: &Option<String>,
    app_caller: &str,
) {
    info!("每日学习任务已启动");
    let r = tokio::runtime::Runtime::new().unwrap();
    let corp_id = corp_id.to_string();
    let corp_secret = corp_secret.to_string();
    let target = target.to_string();
    let proxy_server = proxy_server.clone();
    let app_caller = app_caller.to_string();
    let _ = r
        .spawn(async move {
            let mp = wx::MP::new(corp_id.as_str(), corp_secret.as_str(), agent_id);

            match study::browse_xx(&mp, target.as_str(), &proxy_server, &app_caller).await {
                Ok(_) => {
                    info!("今天的学习强国就逛到这里了");
                }
                Err(e) => {
                    warn!("学习任务执行失败: {}", e);
                }
            };
        })
        .await;
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
