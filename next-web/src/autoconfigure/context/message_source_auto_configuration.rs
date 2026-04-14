use std::error::Error;
#[cfg(not(feature = "embed-resources"))]
use std::sync::Arc;

use next_web_core::{
    async_trait,
    constants::application_constants::MESSAGES,
    context::{
        application_resources::{ApplicationResources, ResourceLoader},
        support::ResourceBundleMessageSource,
        MessageSource,
    },
    traits::config::auto_configuration::AutoConfiguration,
    ApplicationContext,
};

#[cfg(feature = "embed-resources")]
use next_web_core::context::application_resources::RESOURCE_LOADER;
use rudi_dev::singleton;

use crate::autoconfigure::context::message_source_properties::MessageSourceProperties;

#[singleton(binds =[Self::into_auto_configuration])]
#[derive(Clone)]
pub struct MessageSourceAutoConfiguration {
    message_source_properties: MessageSourceProperties,
}

impl MessageSourceAutoConfiguration {
    pub fn into_auto_configuration(self: Self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for MessageSourceAutoConfiguration {
    fn order(&self) -> i32 {
        i32::MIN
    }

    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>> {
        let message_source_single_name: &'static str = "messageSource";

        if ctx.contains_single_with_name::<Arc<dyn MessageSource>>(message_source_single_name) {
            return Ok(());
        }

        #[cfg(feature = "embed-resources")]
        let resource_loader = match RESOURCE_LOADER.get().map(Clone::clone) {
            Some(loader) => loader,
            None => {
                return Err("Please add the macro #[next_application] to the main function".into())
            }
        };

        #[cfg(not(feature = "embed-resources"))]
        let resource_loader: Arc<dyn ResourceLoader> = ctx
            .get_single_with_default_name::<ApplicationResources>()
            .map(Clone::clone)
            .map(Arc::new)
            .ok_or("No `ApplicationResources` found")?;

        // if !condition(&resource_loader, &self.message_source_properties) {
        //     return Ok(());
        // }

        let mut message_source = ResourceBundleMessageSource::new(resource_loader);

        let base_name = self.message_source_properties.base_name();
        if base_name.iter().any(|s| !s.is_empty()) {
            message_source.set_basenames(base_name).await;
        }

        message_source
            .set_fallback_to_system_locale(
                self.message_source_properties.fallback_to_system_locale(),
            )
            .await;
        message_source
            .set_cache_seconds(self.message_source_properties.cache_time() as i64)
            .await;
        message_source.preload_all().await?;

        ctx.insert_singleton_with_name::<Arc<dyn MessageSource>, &'static str>(
            Arc::new(message_source),
            message_source_single_name,
        );
        Ok(())
    }
}

fn condition(
    resource_loader: &Arc<dyn ResourceLoader>,
    message_source_properties: &MessageSourceProperties,
) -> bool {
    message_source_properties
        .base_name()
        .iter()
        .map(|name| format!("{}{}", MESSAGES, name))
        .any(|name| resource_loader.exists(name.as_str()))
}
