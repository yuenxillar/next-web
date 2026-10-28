use std::sync::Arc;

use next_web_context::ApplicationContextExt;
use next_web_core::ApplicationContext;
use next_web_macros::auto_configuration;
use next_web_security::{
    config::{security_builder::SecurityBuilder, web::builders::HttpSecurity},
    web::{
        security_filter_chain::SecurityFilterChain, util::matcher::PathPatternRequestMatcherBuilder,
    },
};

/// Auto-configuration for Web Security.
#[derive(Debug, Clone)]
pub struct SecurityFilterAutoConfiguration;

#[auto_configuration(after = ["WebSecurityAutoConfiguration"])]
impl SecurityFilterAutoConfiguration {
    #[provider( conditional = [Self::on_singleton_name])]
    fn path_pattern_request_matcher_builder() -> PathPatternRequestMatcherBuilder {
        PathPatternRequestMatcherBuilder::default()
    }

    #[provider(name = "securityFilterChain", order = i32::MAX - 5)]
    fn default_security_filter_chain(mut http: HttpSecurity) -> Arc<dyn SecurityFilterChain> {
        http.authorize_http_requests(|requests| {
            requests.any_request().authenticated();
        })
        .form_login(|_| {})
        .http_basic(|_| {});

        Arc::new(http.build())
    }

    fn on_singleton_name(ctx: &dyn ApplicationContext) -> bool {
        false
    }
}
