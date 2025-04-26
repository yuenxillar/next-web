use next_web_core::async_trait;

#[async_trait]
pub trait BaseTopic: Send + Sync {

    /// 
    fn topic(&self) -> &'static str;

    /// 
    async fn consume(&mut self, topic: &str, message: &[u8]);
}