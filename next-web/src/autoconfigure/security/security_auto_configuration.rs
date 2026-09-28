use std::sync::Arc;

use next_web_context::{ApplicationContextExt, ApplicationEventPublisher};
use next_web_core::ApplicationContext;
use next_web_macros::auto_configuration;
use next_web_security::authentication::{
    AuthenticationEventPublisher, DefaultAuthenticationEventPublisher,
};

/// Auto-configuration for Web Security.
#[derive(Debug, Clone)]
pub struct SecurityAutoConfiguration;

#[auto_configuration]
impl SecurityAutoConfiguration {
    #[provider(name = "authenticationEventPublisher", conditional = [Self::on_missing_singleton])]
    fn authentication_event_publisher(
        #[autowired(name = "applicationEventPublisher")] application_event_publisher: Arc<
            dyn ApplicationEventPublisher,
        >,
    ) -> Arc<dyn AuthenticationEventPublisher> {
        Arc::new(DefaultAuthenticationEventPublisher::new(
            application_event_publisher,
        ))
    }

    fn on_missing_singleton(ctx: &dyn ApplicationContext) -> bool {
        !ctx.contains_dyn_singleton_with_default_name::<Arc<dyn AuthenticationEventPublisher>>()
    }
}
