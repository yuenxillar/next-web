use next_web_core::async_trait;

#[async_trait]
pub trait MessageInterceptor: Send + Sync {

    async fn message_entry(&self, topic: &str, data: &[u8]) -> bool;

    // async fn message_push(&self) -> bool;
}
