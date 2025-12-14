use std::sync::Arc;

use crate::{properties::email_properties::EmailProperties, service::email_service::EmailService};
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, traits::singleton::Singleton,
    ApplicationContext, AutoRegister,
};
use rudi_dev::Singleton;

#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct EmailServiceAutoRegister(pub EmailProperties);

impl EmailServiceAutoRegister {
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for EmailServiceAutoRegister {
    fn registered_name(&self) -> &'static str {
        ""
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let email_properties = self.0.clone();

        let email_service = EmailService::new(email_properties)?;
        ctx.insert_singleton_with_name(email_service, self.singleton_name());

        Ok(())
    }
}
