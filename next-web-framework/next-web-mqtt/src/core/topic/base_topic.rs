use next_web_core::async_trait;

#[async_trait]
pub trait BaseTopic: dyn_clone::DynClone + Send + Sync {

    /// 
    fn topic(&self) -> &'static str;

    /// 
    async fn consume(& self, topic: &str, message: &[u8]);
}

dyn_clone::clone_trait_object!(BaseTopic);