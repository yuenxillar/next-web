use async_trait::async_trait;

#[async_trait]
pub trait MiddlewareCheckStatus {
    async fn status(&self) -> Result<(), Box<dyn std::error::Error>>;
}
