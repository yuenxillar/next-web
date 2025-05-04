use async_trait::async_trait;

#[async_trait]
pub trait ApplicationShutdown: Send + Sync {
    fn order(&self) -> i16;

    async fn shutdown(&mut self);
}
