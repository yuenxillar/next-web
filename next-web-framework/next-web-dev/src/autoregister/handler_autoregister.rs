use axum::Router;

use crate::configurer::http_method_handler_configurer::RouterContext;

pub trait HttpHandlerAutoRegister
where Self: Send + Sync +'static
{
    fn register<'a>(&self, __router: Router, __context: &'a mut RouterContext) -> Router;
}

inventory::collect!(&'static dyn HttpHandlerAutoRegister);

#[macro_export]
macro_rules! submit_handler {
    ($ty:ident) => {
        ::next_web_dev::submit! {
            &$ty as &dyn ::next_web_dev::autoregister::handler_autoregister::HttpHandlerAutoRegister
        }
    };
}
