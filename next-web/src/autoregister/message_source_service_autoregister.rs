use crate::service::message_source_service::MessageSourceService;
use next_web_core::{
    async_trait,
    context::{application_resources::ApplicationResources, properties::ApplicationProperties},
    util::singleton::SingletonUtil,
    ApplicationContext, AutoRegister,
};

use super::default_autoregister::DefaultAutoRegister;

pub struct MessageSourceServiceAutoRegister;

#[async_trait]
impl AutoRegister for MessageSourceServiceAutoRegister {
    fn registered_name(&self) -> &'static str {
        ""
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message_source_properties = properties.next().messages().cloned().unwrap_or_default();

        // Retrieve the messages file from the resource
        let application_resources =
            ctx.get_single_with_name(SingletonUtil::name::<ApplicationResources>());

        let message_source_service =
            MessageSourceService::from_resouces(message_source_properties, application_resources);

        ctx.insert_singleton_with_name(message_source_service, "messageSourceService");

        Ok(())
    }
}

impl DefaultAutoRegister for MessageSourceServiceAutoRegister {}

crate::submit! {
    &MessageSourceServiceAutoRegister as &dyn crate::autoregister::default_autoregister::DefaultAutoRegister
}