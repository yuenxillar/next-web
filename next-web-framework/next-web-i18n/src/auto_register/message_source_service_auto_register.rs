use std::{str::FromStr, sync::Arc};

use crate::{
    properties::messages_properties::MessagesProperties,
    service::message_source_service::MessageSourceService, util::locale::Locale,
};
use next_web_core::{
    ApplicationContext, AutoRegister, async_trait,
    constants::{
        application_constants::I18N,
        common_constants::{MESSAGES, PROPERTIES},
    },
    context::{
        application_resources::{ApplicationResources, ResourceLoader},
        properties::ApplicationProperties,
    },
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

        let mut message_source_service = MessageSourceService::new(message_source_properties);

        // Retrieve the messages file from the resource
        let application_resources = ctx.get_single::<ApplicationResources>();

        let base_name = self.0.base_name().unwrap_or(MESSAGES);

        let iters = application_resources.load_dir(I18N);
        iters
            .into_iter()
            .filter(|s| s.ends_with(PROPERTIES))
            .filter(|s| s.starts_with(base_name))
            .for_each(|path| {
                // default
                let locale: Option<Locale> =
                    if path.eq(&format!("{}/{}.{}", I18N, base_name, PROPERTIES)) {
                        Some(Locale::locale())
                    } else {
                        let mut s1 = path
                            .replace(base_name, "")
                            .replace(PROPERTIES, "")
                            .replace(".", "");
                        s1.remove(0);
                        Locale::from_str(s1.as_str()).ok()
                    };

                locale.map(|val| {
                    application_resources.load(path).map(|data| {
                        if let Ok(messages) = String::from_utf8(data.to_vec()) {
                            message_source_service.add_message_source(val, messages)
                        }
                    });
                });
            });

        ctx.insert_singleton_with_name(message_source_service, self.singleton_name());

        Ok(())
    }
}
