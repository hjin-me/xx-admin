use anyhow::Result;
#[async_trait::async_trait]
pub trait Health {
    async fn liveness() -> Result<bool>;
    async fn readiness() -> Result<bool>;
}
