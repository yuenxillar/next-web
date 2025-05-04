use next_web_core::async_trait;

#[async_trait]
pub trait MessageInterceptor: dyn_clone::DynClone + Send + Sync {

    async fn message_entry(&self, topic: &str, data: &[u8]) -> bool;

    // async fn message_push(&self) -> bool;
}


dyn_clone::clone_trait_object!(MessageInterceptor);