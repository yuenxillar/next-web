use next_web_core::{
    async_trait,
    context::properties::ApplicationProperties,
    error::BoxError,
    interface::job::{
        application_job::ApplicationJob, context::job_execution_context::JobExecutionContext,
        schedule_type::ScheduleType,
    },
    ApplicationContext,
};
use next_web_dev::{application::Application, util::local_date_time::LocalDateTime, Singleton};

use std::sync::Arc;

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().route("/", axum::routing::get(|| async move { "Ok" }))
    }
}

#[Singleton(binds=[Self::into_job])]
#[derive(Clone)]
pub struct TestJob;

impl TestJob {
    fn into_job(self) -> Arc<dyn ApplicationJob> {
        Arc::new(self)
    }
}

#[async_trait]
impl ApplicationJob for TestJob {
    fn schedule(&self) -> ScheduleType {
        // from_secs
        ScheduleType::Repeated(2)
    }

    async fn execute(&self, _context: JobExecutionContext) -> Result<(), BoxError> {
        println!("I'm working! time: {}", LocalDateTime::now());
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
