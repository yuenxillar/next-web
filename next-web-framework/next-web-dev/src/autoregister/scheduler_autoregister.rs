use std::future::Future;

use next_web_core::{scheduler::schedule_type::ScheduleType, ApplicationContext};


pub trait SchedulerAutoRegister
where Self: Send + Sync +'static
{
    fn register(&self, __ctx: &mut ApplicationContext) -> AnJob;
}

inventory::collect!(&'static dyn SchedulerAutoRegister);

#[macro_export]
macro_rules! submit_scheduler {
    ($ty:ident) => {
        ::next_web_dev::submit! {
            &$ty as &dyn ::next_web_dev::autoregister::scheduler_autoregister::SchedulerAutoRegister
        }
    };
}


type JobFuture =    Box<dyn Fn() -> std::pin::Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync + 'static>;
type JobFunction =  Box<dyn Fn() + Send + Sync + 'static>;
pub enum AnJob {
    Async((ScheduleType,    JobFuture)),
    Sync((ScheduleType,     JobFunction)),
}
