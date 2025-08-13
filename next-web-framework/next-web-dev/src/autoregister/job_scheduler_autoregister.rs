use crate::manager::job_scheduler_manager::JobSchedulerManager;
use next_web_core::async_trait;
use next_web_core::context::properties::ApplicationProperties;
use next_web_core::{ApplicationContext, AutoRegister};

#[derive(Default)]
pub struct JobSchedulerAutoRegister;

#[async_trait]
impl AutoRegister for JobSchedulerAutoRegister {
    fn registered_name(&self) -> &'static str {
        ""
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut scheduler_manager = JobSchedulerManager::new();
        scheduler_manager.start();
        
        ctx.insert_singleton(scheduler_manager);
        Ok(())
    }
}
