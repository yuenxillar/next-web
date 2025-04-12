use crate::manager::job_scheduler_manager::JobSchedulerManager;
use next_web_core::async_trait;
use next_web_core::context::properties::ApplicationProperties;
use next_web_core::{ApplicationContext, AutoRegister};

#[derive(Default)]
pub struct JobSchedulerAutoRegister;

#[async_trait]
impl AutoRegister for JobSchedulerAutoRegister {
    fn singleton_name(&self) -> &'static str {
        "jobSchedulerManager"
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let job_scheduler_manager = JobSchedulerManager::new();
        ctx.insert_singleton_with_name(job_scheduler_manager, self.singleton_name());

        Ok(())
    }
}
