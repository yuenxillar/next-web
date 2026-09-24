use next_web_context::ApplicationContextExt;
use next_web_core::{
    async_trait, traits::config::auto_configuration::AutoConfiguration, ApplicationContext, Ordered,
};
use next_web_macros::singleton;
use std::{error::Error, sync::Arc};

use crate::i18n::{AcceptHeaderLocaleResolver, LocaleResolver};

#[derive(Clone)]
#[singleton(binds =[Self::into_auto_configuration])]
pub struct WebAutoConfiguration;

impl WebAutoConfiguration {
    fn locale_resolver(&self, ctx: &mut dyn ApplicationContext) {
        if ctx.contains_singleton_with_name::<Arc<dyn LocaleResolver>>("localeResolver") {
            ctx.insert_singleton_with_name(
                Arc::new(AcceptHeaderLocaleResolver::default()),
                "localeResolver",
            );
        }
    }

    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for WebAutoConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        self.locale_resolver(ctx);

        Ok(())
    }
}

impl Ordered for WebAutoConfiguration {
    fn order(&self) -> i32 {
        i32::MIN + 10
    }
}
