use crate::state::XxState;
use crate::XxManagerPool;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use tracing::instrument;
use study_core::utils::UserValidator;

#[derive(Clone)]
pub struct StateSession<T: UserValidator + Clone + Sync + Send + 'static> {
    data: Arc<RwLock<HashMap<u64, XxState>>>,
    counter: Arc<AtomicU64>,
    pool: XxManagerPool<T>,
}

impl<T: UserValidator + Clone + Send + Sync + 'static> StateSession<T> {
    pub fn new(pool: XxManagerPool<T>) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            counter: Arc::new(AtomicU64::new(0)),
            pool,
        }
    }

    #[instrument(skip(self), level = "trace")]
    pub fn get(&self, id: u64) -> Option<XxState> {
        let data = self.data.read().unwrap();
        data.get(&id).map(|s| s.clone())
    }

    #[instrument(skip(self), level = "trace")]
    pub fn new_state(&self) -> Result<u64> {
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut data = self.data.write().unwrap();
        let state = XxState::new();
        state.serve(self.pool.clone())?;
        data.insert(id, state);
        Ok(id)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::XxManager;
    use anyhow::anyhow;
    use std::time::Duration;
    use study_core::State;
    use tracing::info;

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    async fn test_state() -> Result<()> {
        tracing_subscriber::fmt::init();
        let manager = XxManager::new();
        let pool = bb8::Pool::builder()
            .max_size(2)
            .min_idle(Some(1))
            .idle_timeout(Some(Duration::from_secs(170)))
            // .connection_timeout(std::time::Duration::from_secs(30))
            .build(manager)
            .await
            .unwrap();

        let ss = StateSession::new(&pool);
        let s_id = ss.new_state()?;
        loop {
            {
                let s = ss.get(s_id).ok_or(anyhow!("没有找到状态数据"))?;
                let is = s.get_state();

                info!("读取状态数据 {:?}", is);
                if let State::Complete(_) = is {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
        Ok(())
    }
}
