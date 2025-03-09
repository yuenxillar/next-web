use async_trait::async_trait;

#[async_trait]
pub trait RedisExpiredEvent: Send + Sync + 'static {
    async fn on_message(&mut self, message: String, pattern: String);
}
