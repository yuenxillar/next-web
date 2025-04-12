use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext, AutoRegister};
use rudi::SingleOwner;

use crate::{properties::email_properties::EmailProperties, service::email_service::EmailService};

#[SingleOwner(binds = [Self::into_auto_register])]
pub struct EmailServiceAutoRegister(pub EmailProperties);


impl EmailServiceAutoRegister {
    fn into_auto_register(self) -> Box<dyn AutoRegister> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoRegister for EmailServiceAutoRegister {
    
    fn singleton_name(&self) -> &'static str {
        "emailService"
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let  email_properties = self.0.clone();

        let email_service = EmailService::new(email_properties)?;
        ctx.insert_singleton_with_name(email_service, self.singleton_name());

        Ok(())
    }
}