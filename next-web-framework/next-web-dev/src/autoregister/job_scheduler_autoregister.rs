use crate::autoregister::auto_register::AutoRegister;
use crate::manager::job_scheduler_manager::JobSchedulerManager;

#[derive(Default)]
pub struct JobSchedulerAutoRegister;

impl AutoRegister for JobSchedulerAutoRegister {

    fn name(&self) -> &'static str {
        "JobSchedulerAutoRegister"
    }
    fn register(&self, ctx: &mut rudi::Context) -> Result<(), Box<dyn std::error::Error>> {
        let job_scheduler_manager = JobSchedulerManager::new();
        ctx.insert_singleton_with_name::<JobSchedulerManager, String>(
            job_scheduler_manager,
            String::from("jobSchedulerManager"),
        );
        println!("JobSchedulerAutoRegister registered successfully!");
        Ok(())
    }
}
