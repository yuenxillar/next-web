use std::sync::Arc;

use next_web_core::{
    ApplicationContext, async_trait, error::BoxError,
    traits::config::auto_configuration::AutoConfiguration,
};
use rudi_dev::singleton;

use crate::mail::{
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
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), BoxError> {
        let mail_properties = self.mail_properties.clone();

        let default_mail_service = DefaultMailService::new(mail_properties)?;

        default_mail_service.test_connection().await;

        ctx.insert_singleton_with_name::<Arc<dyn MailService>, &'static str>(
            Arc::new(default_mail_service),
            "mailService",
        );

        Ok(())
    }
}
