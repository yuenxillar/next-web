use crate::ApplicationContext;

pub trait DefaultAutoConfigure
where
    Self: Send + Sync,
    Self: 'static,
{
    fn auto_configure<'life_a>(
        &'life_a self,
        ctx: &'life_a mut ApplicationContext,
    ) -> core::pin::Pin<std::boxed::Box<dyn Future<Output = ()> + Send + 'life_a>>;
}

inventory::collect!(&'static dyn DefaultAutoConfigure);

#[macro_export]
macro_rules! submit_default_auto_configure {
    ($ty:ident) => {
        ::next_web::submit! {
            &$ty as &dyn ::next_web_core::autoconfigure::default_auto_configure::DefaultAutoConfigure
        }
    };
}
