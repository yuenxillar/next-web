use std::future::Future;

use next_web_core::ApplicationContext;


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

pub enum AnJob {
    Async(Box<dyn Fn() -> std::pin::Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync + 'static>),
    Sync(Box<dyn Fn() + Send + Sync + 'static>),
}
