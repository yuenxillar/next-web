use std::sync::Arc;

use next_web_core::ApplicationContext;
use next_web_core::scheduler::ScheduledJobHandler;

use crate::manager::job_scheduler_manager::BoxedJob;

pub trait SchedulerAutoRegister
where
    Self: Send + Sync + 'static,
{
    fn register(&self, __ctx: &mut ApplicationContext) -> BoxedJob;
}

inventory::collect!(&'static dyn SchedulerAutoRegister);

pub trait ScheduledJobHandlerAutoRegister
where
    Self: Send + Sync + 'static,
{
    fn register(&self, __ctx: &mut ApplicationContext) -> Arc<dyn ScheduledJobHandler>;
}

inventory::collect!(&'static dyn ScheduledJobHandlerAutoRegister);

#[macro_export]
macro_rules! submit_scheduler {
    ($ty:ident) => {
        ::next_web::submit! {
            &$ty as &dyn ::next_web::autoregister::scheduler_autoregister::SchedulerAutoRegister
        }
    };
}

#[macro_export]
macro_rules! submit_scheduled_job_handler {
    ($ty:ident) => {
        ::next_web::submit! {
            &$ty as &dyn ::next_web::autoregister::scheduler_autoregister::ScheduledJobHandlerAutoRegister
        }
    };
}
