use next_web_context::ApplicationContextExt;
use std::{error::Error, sync::Arc};

use next_web_core::{
    ApplicationContext, Ordered, async_trait, traits::config::auto_configuration::AutoConfiguration,
};
use next_web_macros::singleton;

use crate::{
    autoconfigure::mail_properties::MailProperties, mail_service::MailService,
    service::default_mail_service::DefaultMailService,
};

/// Auto-configuration for Mail.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct MailAutoConfiguration {
    pub mail_properties: MailProperties,
}

impl MailAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for MailAutoConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        let mail_properties = self.mail_properties.clone();

        let default_mail_service = DefaultMailService::new(mail_properties)?;

        default_mail_service.test_connection().await?;

        ctx.insert_singleton_with_name::<Arc<dyn MailService>>(
            Arc::new(default_mail_service),
            "mailService",
        );

        Ok(())
    }
}

impl Ordered for MailAutoConfiguration {
    fn order(&self) -> i32 {
        100
    }
}
