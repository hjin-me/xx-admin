///#![allow(clippy::needless_doctest_main)]
/// #![deny(missing_docs, missing_debug_implementations)]
use crate::xx::Xx;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
pub use bb8;
//
#[derive(Clone, Debug)]
pub struct XxManager {
    app_caller: String,
}

impl XxManager {
    pub fn new(app_caller: &str) -> Self {
        Self {
            app_caller: app_caller.to_string(),
        }
    }
    pub async fn get_one(&self) -> Result<Xx> {
        let app_caller = self.app_caller.clone();
        tokio::spawn(async move { Xx::new(&app_caller) }).await?
    }
}

#[async_trait]
impl bb8::ManageConnection for XxManager {
    type Connection = Xx;
    type Error = anyhow::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.get_one().await
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        if conn.ping() {
            Ok(())
        } else {
            Err(anyhow!("连接已经断开"))
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        !conn.ping()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread;
    use std::thread::spawn;
    use std::time::Duration;
    use tracing::{error, info};
    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn test_pool() {
        tracing_subscriber::fmt::init();
        let manager = XxManager {
            app_caller: "https://techxuexi.js.org/jump/techxuexi-20211023.html?".to_string(),
        };
        let pool = bb8::Pool::builder()
            .max_size(4)
            .min_idle(Some(2))
            .connection_timeout(Duration::from_secs(30))
            .build(manager)
            .await
            .unwrap();

        let mut handles = vec![];
        tokio::time::sleep(Duration::from_secs(30)).await;

        for _i in 0..5 {
            let pool = pool.clone();
            handles.push(spawn(move || {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async move {
                        info!("new one spawn");
                        let mut conn = match pool.get().await {
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
                                return;
                            }
                        };
                        info!("new spawn get poll");

                        let ticket = conn.get_ticket();
                        info!("ticket = {}", ticket);

                        loop {
                            match conn.check_login() {
                                Ok(b) => {
                                    if b {
                                        info!("登陆成功");
                                        break;
                                    } else {
                                        info!("还没登陆")
                                    }
                                }
                                Err(e) => {
                                    error!("判断登陆状态失败: {}", e);

                                    break;
                                }
                            }
                            thread::sleep(Duration::from_secs(10));
                        }
                        let news_list = crate::get_news_list().await.expect("获取新闻列表失败");
                        let video_list = crate::get_video_list().await.expect("获取视频列表失败");
                        _ = dbg!(conn.try_study(&news_list, &video_list));
                    });
            }));
        }

        for x in handles {
            _ = x.join();
        }
    }
}
