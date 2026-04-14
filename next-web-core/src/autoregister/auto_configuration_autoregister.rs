use std::error::Error;

use async_trait::async_trait;

use crate::ApplicationContext;

#[async_trait]
pub trait DefaultAutoConfigurationAutoregister
where
    Self: Send + Sync,
    Self: 'static,
{
    async fn configuration(&self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>>;
}

inventory::collect!(&'static dyn DefaultAutoConfigurationAutoregister);

#[macro_export]
macro_rules! submit_default_auto_configure {
    ($ty:ident) => {
        ::next_web_core::submit! {
            &$ty as &dyn ::next_web_core::autoregister::auto_configuration_autoregister::DefaultAutoConfigurationAutoregister
        }
    };
}
