use async_trait::async_trait;

use crate::ApplicationContext;

#[async_trait]
pub trait ApplicationReadyEvent: Send + Sync {
    async fn ready(&self, ctx: &mut ApplicationContext);
}
