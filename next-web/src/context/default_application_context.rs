use std::sync::Arc;

use next_web_context::{
    ApplicationContext, ApplicationEvent, ApplicationEventPublisher, ApplicationListener,
    MessageSource,
};
use next_web_core::{env::ConfigurableEnvironment, error::BoxError, metrics::ApplicationStartup};
use next_web_singletons::factory::support::DefaultListableSingletonFactory;

use crate::{configurable_application_context::ContextError, ConfigurableApplicationContext};

#[derive(Default)]
pub struct DefaultApplicationContext {}

impl ConfigurableApplicationContext for DefaultApplicationContext {
    fn set_id(&mut self, id: String) {
        todo!()
    }

    fn set_parent(&mut self, parent: Option<Arc<dyn ApplicationContext>>) {
        todo!()
    }

    fn set_environment(&mut self, environment: Arc<dyn ConfigurableEnvironment>) {
        todo!()
    }

    fn environment(&self) -> &dyn ConfigurableEnvironment {
        todo!()
    }

    fn set_application_startup(&mut self, application_startup: Box<dyn ApplicationStartup>) {
        todo!()
    }

    fn application_startup(&self) -> &dyn ApplicationStartup {
        todo!()
    }
    fn add_application_listener(
        &mut self,
        listener: Box<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    ) {
        todo!()
    }

    fn remove_application_listener(&mut self, id: &str) {
        todo!()
    }

    fn refresh(&mut self) -> Result<(), ContextError> {
        todo!()
    }

    fn restart(&mut self) -> Result<(), ContextError> {
        todo!()
    }

    fn pause(&mut self) -> Result<(), ContextError> {
        todo!()
    }

    fn register_shutdown_hook(&mut self) {
        todo!()
    }

    fn close(&mut self) -> Result<(), ContextError> {
        todo!()
    }

    fn is_closed(&self) -> bool {
        todo!()
    }

    fn is_active(&self) -> bool {
        todo!()
    }

    fn parent(&self) -> Option<&dyn ApplicationContext> {
        todo!()
    }

    fn singleton_factory(&mut self) -> Result<&mut DefaultListableSingletonFactory, ContextError> {
        todo!()
    }
}

impl ApplicationContext for DefaultApplicationContext {
    fn application_name(&self) -> &str {
        todo!()
    }

    fn id(&self) -> &str {
        todo!()
    }

    fn startup_date(&self) -> i64 {
        todo!()
    }
}

impl ApplicationEventPublisher for DefaultApplicationContext {
    fn publish_event(
        &self,
        event: Box<dyn next_web_context::ApplicationEvent>,
    ) -> Result<(), BoxError> {
        todo!()
    }
}

impl MessageSource for DefaultApplicationContext {
    fn message(
        &self,
        code: &str,
        args: Option<&[&dyn std::fmt::Display]>,
        locale: Option<&next_web_context::Locale>,
    ) -> Result<String, next_web_context::NoSuchMessageError> {
        todo!()
    }

    fn message_from_resolvable(
        &self,
        resolvable: &dyn next_web_context::MessageSourceResolvable,
        locale: Option<&next_web_context::Locale>,
    ) -> Result<String, next_web_context::NoSuchMessageError> {
        todo!()
    }

    fn message_or_default(
        &self,
        code: &str,
        args: Option<&[&dyn std::fmt::Display]>,
        default_message: Option<&str>,
        locale: Option<&next_web_context::Locale>,
    ) -> Option<String> {
        todo!()
    }
}
