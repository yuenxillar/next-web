use std::sync::Arc;

use next_web_context::ApplicationContextExt;
use next_web_core::{ApplicationContext, env::ConfigurableEnvironment};
use next_web_macros::auto_configuration;
use next_web_security::{
    config::{security_builder::SecurityBuilder, web::builders::HttpSecurity},
    web::{security_filter_chain::SecurityFilterChain, util::matcher::RequestMatcher},
};

/// Auto-configuration for Web Security when actuator is on the classpath.
/// It allows unauthenticated access to the HealthEndpoint. If the user specifies
/// their own SecurityFilterChain bean, this will back-off completely and the user should
/// specify all the bits that they want to configure as part of the custom security configuration.
#[derive(Debug, Clone)]
pub struct ManagementWebSecurityAutoConfiguration;

#[auto_configuration(before = ["WebSecurityAutoConfiguration"],conditional = [Self::on_missing_singleton, super::web::on_default_web_security])]
impl ManagementWebSecurityAutoConfiguration {
    #[provider(name = "securityFilterChain", order = i32::MAX - 5)]
    fn management_security_filter_chain(
        #[autowired(name = "applicationEnvironment")] _environment: Arc<
            dyn ConfigurableEnvironment,
        >,
        mut http: HttpSecurity,
    ) -> Arc<dyn SecurityFilterChain> {
        http.cors(|_| {}).form_login(|_| {}).http_basic(|_| {});

        Arc::new(http.build())
    }

    fn on_missing_singleton(ctx: &dyn ApplicationContext) -> bool {
        !ctx.contains_dyn_singleton_with_default_name::<Arc<dyn RequestMatcher>>()
    }
}
