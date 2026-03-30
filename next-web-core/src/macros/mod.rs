#[macro_export]
macro_rules! id {
    ($listener:ty, $event:ty) => {
        concat!(stringify!($listener), "<", stringify!($event), ">")
    };
}

#[macro_export]
macro_rules! impl_service {
    ($service:ident) => {
        impl ::next_web_core::traits::singleton::Singleton for $service {}
        impl ::next_web_core::traits::service::Service for $service {}
    };
}
