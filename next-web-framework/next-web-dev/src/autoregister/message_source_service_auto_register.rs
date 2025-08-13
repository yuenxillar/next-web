use next_web_core::{
    async_trait,
    context::{application_resources::ApplicationResources, properties::ApplicationProperties},
    ApplicationContext, AutoRegister,
};
use tracing::warn;

use crate::service::message_source_service::MessageSourceService;

#[derive(Default)]
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
        let message_source_properties = properties.next().messages();

        if let None = message_source_properties.as_ref() {
            warn!("No message source properties found");
            return Ok(());
        }

        // Retrieve the messages file from the resource
        let application_resources = ctx.get_single::<ApplicationResources>();

        let message_source_service =
            MessageSourceService::from_resouces(message_source_properties.unwrap().to_owned(), application_resources);

        ctx.insert_singleton_with_name(message_source_service, "messageSourceService");

        Ok(())
    }
}
