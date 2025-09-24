use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application,
    scheduler::{context::JobExecutionContext, schedule_type::ScheduleType},
    traits::schedule::scheduled_task::ScheduledTask,
    util::local_date_time::LocalDateTime,
    Scheduled, Singleton,
};

use std::sync::Arc;

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    async fn before_start(&mut self, _ctx: &mut ApplicationContext) {}
}

#[Singleton(binds=[Self::into_task])]
#[derive(Clone)]
pub struct TestJob;

impl TestJob {
    fn into_task(self) -> Arc<dyn ScheduledTask> {
        Arc::new(self)
    }
}

#[async_trait]
impl ScheduledTask for TestJob {
    fn schedule(&self) -> ScheduleType {
        // from  seconds
        ScheduleType::Repeated(2)
    }

    async fn execute(&self, _context: JobExecutionContext) {
        println!("I'm working! time: {}", LocalDateTime::now());
    }
}

#[Scheduled(fixed_rate = 1000)]
async fn test_scheduled() {
    println!("I'm working! time: {}", LocalDateTime::now());
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
