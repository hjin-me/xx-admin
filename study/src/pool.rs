///#![allow(clippy::needless_doctest_main)]
/// #![deny(missing_docs, missing_debug_implementations)]
use anyhow::{anyhow, Result};
use async_trait::async_trait;
pub use bb8;
use study_core::utils::UserValidator;
use study_core::Xx;
use tracing::debug;
//

pub type XxManagerPool<T> = bb8::Pool<XxManager<T>>;
#[derive(Clone, Debug)]
pub struct XxManager<T: UserValidator + Send + Sync + Clone> {
    uv: T,
    proxy_server: Option<String>,
}

impl<T: UserValidator + Send + Sync + Clone + 'static> XxManager<T> {
    pub fn new(v: T, proxy_server: Option<String>) -> Self {
        Self {
            uv: v.clone(),
            proxy_server,
        }
    }
    pub async fn get_one(&self) -> Result<Xx> {
        let uv = self.uv.clone();
        let proxy_server = self.proxy_server.clone();
        tokio::spawn(async move { Xx::new(uv, proxy_server) }).await?
    }
}

#[async_trait]
impl<T: UserValidator + Send + Sync + Clone + 'static> bb8::ManageConnection for XxManager<T> {
    type Connection = Xx;
    type Error = anyhow::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.get_one().await
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        if conn.is_valid() {
            debug!("connect is valid");
            Ok(())
        } else {
            debug!("connect is invalid");
            Err(anyhow!("连接已经断开"))
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        let r = conn.ping();
        debug!("connect ping = {}", r);
        !r
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::spawn;
    use std::time::Duration;
    use sysinfo::{ProcessExt, System, SystemExt};
    use tracing::{error, info};

    struct MockUV {}

    impl UserValidator for MockUV {
        async fn validate(&self, uid: i64) -> Result<bool> {
            Ok(true)
        }
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pool() {
        tracing_subscriber::fmt::init();
        let mut system = System::new_all();

        // 更新所有进程信息
        system.refresh_processes();
        info!("=========before========");
        let before_pids_count = system
            .processes()
            .iter()
            .filter(|(pid, process)| {
                if process.name().contains("hrome") {
                    info!("pid: {}, name: {}", pid, process.name());
                    !process.name().contains("crashpad")
                } else {
                    false
                }
            })
            .count();

        let manager = XxManager::new(MockUV {}, None);
        let pool = bb8::Pool::builder()
            .max_size(2)
            .min_idle(Some(1))
            .connection_timeout(Duration::from_secs(60))
            .build(manager)
            .await
            .unwrap();

        let mut handles = vec![];
        tokio::time::sleep(Duration::from_secs(30)).await;

        for _i in 0..2 {
            let pool = pool.clone();
            handles.push(spawn(move || {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async move {
                        info!("new one spawn");
                        let conn = match pool.get().await {
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
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        info!("获取链接成功了，这个时候我把连接丢掉");
                    });
            }));
        }

        for x in handles {
            _ = x.join();
        }
        drop(pool);
        info!("=========sleep========");
        tokio::time::sleep(Duration::from_secs(30)).await;
        // 更新所有进程信息
        system.refresh_processes();
        info!("=========after========");
        let after_pids_count = system
            .processes()
            .iter()
            .filter(|(pid, process)| {
                if process.name().contains("hrome") {
                    info!("pid: {}, name: {}", pid, process.name());
                    !process.name().contains("crashpad")
                } else {
                    false
                }
            })
            .count();
        assert_eq!(after_pids_count, before_pids_count);
    }
}
