use crate::manager::job_scheduler_manager::JobSchedulerManager;
use next_web_core::async_trait;
use next_web_core::context::properties::ApplicationProperties;
use next_web_core::core::singleton::Singleton;
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
        let job_scheduler_manager = JobSchedulerManager::new();

        let singleton_name = job_scheduler_manager.singleton_name();
        ctx.insert_singleton_with_name(job_scheduler_manager, singleton_name);

        Ok(())
    }
}
