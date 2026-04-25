use std::sync::Arc;

use next_web::{
    application::Application,
    core::{ApplicationContext, async_trait, context::properties::ApplicationProperties},
    macros::bind::singleton,
};
use next_web_core::error::BoxError;
use next_web_xxl_job::executor::context::job_context::{AsyncJobHandler, JobContext};
use tracing::info;

#[derive(Clone, Default)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[derive(Clone)]
#[singleton(binds = [Self::into_job])]
pub struct TestJob;

impl TestJob {
    fn into_job(self: Self) -> Arc<dyn AsyncJobHandler> {
        Arc::new(self)
    }
}

#[async_trait]
impl AsyncJobHandler for TestJob {
    fn name(&self) -> String {
        "TestJob".to_string()
    }

    async fn process(&self, context: JobContext) -> Result<JobContext, BoxError> {
        info!("Ok");
        Ok(context)
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
