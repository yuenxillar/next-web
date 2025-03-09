use async_trait::async_trait;

#[async_trait]
pub trait ApplicationShutdown: Send + Sync {
    fn order(&self) -> u16;

    async fn shutdown(&self);
}
