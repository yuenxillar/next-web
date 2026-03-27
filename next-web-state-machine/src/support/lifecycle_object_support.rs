use next_web_core::{async_trait, error::BoxError};

#[derive(Clone)]
pub struct LifecycleObjectSupport {}

impl LifecycleObjectSupport {
    pub fn on_init(&mut self) -> Result<(), BoxError> {
        todo!()
    }
}

impl Default for LifecycleObjectSupport {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait]
pub trait LifecycleObjectSupportExt {
    fn on_init(&mut self) -> Result<(), BoxError> {
        Ok(())
    }

    async fn do_destroy(&self) {}

    async fn do_pre_start(&mut self) {}

    async fn do_pre_stop(&mut self) {}

    async fn do_post_start(&self) {}
}
