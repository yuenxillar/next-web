use next_web_core::ApplicationContext;

use crate::manager::job_scheduler_manager::BoxedJob;

pub trait SchedulerAutoRegister
where
    Self: Send + Sync + 'static,
{
    fn register(&self, __ctx: &mut ApplicationContext) -> BoxedJob;
}

inventory::collect!(&'static dyn SchedulerAutoRegister);

#[macro_export]
macro_rules! submit_scheduler {
    ($ty:ident) => {
        ::next_web::macros::submit! {
            &$ty as &dyn ::next_web::autoregister::scheduler_autoregister::SchedulerAutoRegister
        }
    };
}