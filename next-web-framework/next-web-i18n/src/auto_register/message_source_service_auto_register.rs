use std::sync::Arc;

use crate::{
    properties::messages_properties::MessagesProperties,
    service::message_source_service::MessageSourceService,
};
use next_web_core::{
    ApplicationContext, AutoRegister, async_trait, context::properties::ApplicationProperties,
    interface::singleton::Singleton,
};
use rudi_dev::Singleton;

#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct MessageSourceServiceAutoRegister(pub MessagesProperties);

impl MessageSourceServiceAutoRegister {
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for MessageSourceServiceAutoRegister {
    fn registered_name(&self) -> &'static str {
        ""
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message_source_properties = self.0.clone();

        let message_source_service = MessageSourceService::new(message_source_properties);
        ctx.insert_singleton_with_name(message_source_service, self.singleton_name());

        Ok(())
    }
}
