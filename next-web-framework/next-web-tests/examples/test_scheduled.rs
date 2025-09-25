use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application,
    scheduler::{context::JobExecutionContext, schedule_type::{ScheduleType, WithArgs}},
    traits::schedule::scheduled_task::ScheduledTask,
    util::local_date_time::LocalDateTime,
    Scheduled, Singleton,
};

use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}
    async fn before_start(&mut self, _ctx: &mut ApplicationContext) {}
}

#[Singleton(binds=[Self::into_task])]
#[derive(Clone)]
pub struct TestTask {
    #[autowired(default)]
    pub number: Arc<AtomicI32>,
}

impl TestTask {
    fn into_task(self) -> Arc<dyn ScheduledTask> {
        Arc::new(self)
    }
}

#[async_trait]
impl ScheduledTask for TestTask {
    fn schedule(&self) -> ScheduleType {
        // from  seconds
        ScheduleType::FixedRate(WithArgs::default())
    }

    async fn execute(&self, _context: JobExecutionContext) {
        println!("I'm working! time: {}", LocalDateTime::now());
    }
}

// Cron Scheduled Task
// Local or Utc or .....
#[Scheduled(cron = "*/3 * * * * *", timezone = "Asia/Shanghai")]
async fn test_cron_scheduled() {
    println!(
        "Cron Scheduled Task!       time: {}",
        LocalDateTime::now()
    );
}

// Fixed Rate Scheduled Task
#[Scheduled(fixed_rate = 2000, time_unit = "ms")]
async fn test_fixed_rate_scheduled() {
    println!(
        "Fixed Rate Scheduled Task! time: {}",
        LocalDateTime::now()
    );
}

// One Shot Scheduled Task
// time_unit   Milliseconds or ms
#[Scheduled(one_shot, initial_delay = 5000, time_unit = "Milliseconds")]
async fn test_one_shot_scheduled(test: TestTask) {
    test.number.fetch_add(1, Ordering::Relaxed);
    println!(
        "One Shot Scheduled Task! number: {}",
        test.number.load(Ordering::Relaxed)
    );
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
