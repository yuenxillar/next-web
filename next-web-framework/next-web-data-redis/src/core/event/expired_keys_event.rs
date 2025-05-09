use next_web_core::async_trait;
use next_web_core::DynClone;

#[async_trait]
pub trait RedisExpiredKeysEvent: DynClone + Send + Sync {
    async fn on_message(&mut self, message: &[u8], pattern: &[u8]);
}

next_web_core::clone_trait_object!(RedisExpiredKeysEvent);