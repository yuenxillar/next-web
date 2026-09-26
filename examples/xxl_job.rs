use std::sync::Arc;

use next_web::{Application, NextWebApplication, core::async_trait, macros::bind::singleton};
use next_web_core::error::BoxError;
use next_web_xxl_job::executor::context::job_context::{AsyncJobHandler, JobContext};
use tracing::info;

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

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
    NextWebApplication::<TestApplication>::default().run().await
}
