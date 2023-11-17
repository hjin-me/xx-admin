use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use study::bb8::Pool;
use study::{bb8, XxManager};
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
pub async fn run(pool: Pool<XxManager>) -> Result<String> {
    info!("{:?}", tokio::runtime::Handle::current().runtime_flavor());

    info!("pool get");
    let mut conn = match pool.get_owned().await {
        Ok(conn) => conn,
        Err(e) => {
            match e {
                bb8::RunError::User(e) => {
                    error!("获取连接失败: {}", e);
                }
                bb8::RunError::TimedOut => {
                    error!("获取连接超时");
                }
            }
            return Err(anyhow!("获取连接池失败了"));
        }
    };
    info!("got");

    let ticket = conn.get_ticket();
    info!("ticket = {}", ticket);
    thread::spawn(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async move {
            tokio::select! {
               _ = async {loop {
                match conn.check_login() {
                    Ok(b) => if b {
                        break;
                    } else {
                        trace!("还没登陆");
                        sleep(Duration::from_secs(5)).await;
                    },
                    Err(e) => {
                        debug!("判断登陆状态失败: {}", e);
                    }
                }
                }

                   let news_list = vec!["https://www.xuexi.cn/lgpage/detail/index.html?id=1675585234174641917&item_id=1675585234174641917".to_string()];
            let video_list :Vec<String>= vec![];
            match conn.try_study(&news_list, &video_list) {
                Ok(_) => {
                    info!("学习成功");
                }
                Err(e) => {
                    error!("学习失败: {}", e);
                }
            }
            } => {},
               _ = time::sleep(Duration::from_secs(30)) => {
                    warn!("等待登陆超时");
                },
            }
            drop(conn)
       })
    });

    let mut app_caller = "".to_string();
    app_caller.extend(form_urlencoded::byte_serialize(ticket.as_bytes()));
    Ok(app_caller)
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
