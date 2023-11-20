use axum::async_trait;
use serde::Deserialize;
use study_core::utils::UserValidator;

#[derive(Clone, Default)]
pub struct WBList {
    w: Option<Vec<i64>>,
    b: Option<Vec<i64>>,
}

#[async_trait]
impl UserValidator for WBList {
    async fn validate(&self, uid: i64) -> anyhow::Result<bool> {
        if let Some(b) = &self.b {
            if b.contains(&uid) {
                return Ok(false);
            }
        }
        if let Some(w) = &self.w {
            return Ok(w.contains(&uid));
        }
        Ok(true)
    }
}
