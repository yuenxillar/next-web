use axum::Router;

pub trait HttpHandlerAutoRegister
where Self: Send + Sync +'static
{
    fn register(&self, __router: Router) -> Router;
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
