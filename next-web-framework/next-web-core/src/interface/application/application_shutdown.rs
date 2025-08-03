use async_trait::async_trait;
use dyn_clone::DynClone;

#[async_trait]
pub trait ApplicationShutdown: DynClone + Send + Sync {

    fn order(&self) -> i16;

    async fn shutdown(&mut self);
}

dyn_clone::clone_trait_object!(ApplicationShutdown);