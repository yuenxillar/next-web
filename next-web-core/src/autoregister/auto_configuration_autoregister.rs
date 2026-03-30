use async_trait::async_trait;

use crate::{ApplicationContext, error::BoxError};

#[async_trait]
pub trait DefaultAutoConfigurationAutoregister
where
    Self: Send + Sync,
    Self: 'static,
{
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), BoxError>;
}

inventory::collect!(&'static dyn DefaultAutoConfigurationAutoregister);

#[macro_export]
macro_rules! submit_default_auto_configure {
    ($ty:ident) => {
        ::next_web::submit! {
            &$ty as &dyn ::next_web_core::autoregister::auto_configuration_autoregister::DefaultAutoConfigurationAutoregister
        }
    };
}
