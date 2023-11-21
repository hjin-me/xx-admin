use crate::backend::conf::BaseConf;
use axum::async_trait;
use serde::Deserialize;
use study_core::utils::UserValidator;
use tracing::debug;

#[derive(Clone, Default)]
pub struct WBList {
    conf_path: String,
}
impl WBList {
    pub fn new(conf_path: &str) -> Self {
        let conf = BaseConf::from_path(conf_path).expect("读取配置文件失败");
        Self {
            conf_path: conf_path.to_string(),
        }
    }
}

#[async_trait]
impl UserValidator for WBList {
    async fn validate(&self, uid: i64) -> anyhow::Result<bool> {
        let conf = BaseConf::from_path(&self.conf_path)?;
        if let Some(b) = &conf.black_list {
            if b.contains(&uid) {
                debug!("uid {} in black list", uid);
                return Ok(false);
            }
        }
        if let Some(w) = &conf.white_list {
            return Ok(if w.contains(&uid) {
                true
            } else {
                debug!("uid {} not in white list", uid);
                false
            });
        }
        Ok(true)
    }
}
