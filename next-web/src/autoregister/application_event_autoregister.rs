use futures::future::BoxFuture;
use next_web_context::ApplicationContext;

pub trait ApplicationEventAutoRegister
where
    Self: Send + Sync,
    Self: 'static,
{
    fn register<'life_a>(
        &'life_a self,
        ctx: &'life_a mut dyn ApplicationContext,
    ) -> BoxFuture<'life_a, ()>;
}

inventory::collect!(&'static dyn ApplicationEventAutoRegister);

#[macro_export]
macro_rules! submit_application_event_autoregister {
    ($ty:ident) => {
        ::next_web::macros::submit! {
            &$ty as &dyn ::next_web::autoregister::application_event_autoregister::ApplicationEventAutoRegister
        }
    };
}
