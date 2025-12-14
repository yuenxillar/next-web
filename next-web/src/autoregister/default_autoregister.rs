use next_web_core::AutoRegister;

pub trait DefaultAutoRegister
where 
Self:   Send   + Sync + 'static,
Self:   AutoRegister
{}

inventory::collect!(&'static dyn DefaultAutoRegister);

#[macro_export]
macro_rules! submit_default_autoregister {
    ($ty:ident) => {
        ::next_web::submit! {
            &$ty as &dyn ::next_web::autoregister::default_autoregister::DefaultAutoRegister
        }
    };
}