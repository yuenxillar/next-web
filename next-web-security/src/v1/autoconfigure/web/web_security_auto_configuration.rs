use std::sync::Arc;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        web::{
            builders::HttpSecurity, configurers::AuthorizeHttpRequestsConfigurer,
            HttpSecurityBuilder,
        },
    },
    web::{
        security_filter_chain::SecurityFilterChain, util::matcher::PathPatternRequestMatcherBuilder,
    },
};
use next_web_context::ApplicationContextExt;
use next_web_core::ApplicationContext;
use next_web_macros::auto_configuration;

/// Auto-configuration for Web Security.
#[derive(Debug, Clone)]
pub struct WebSecurityAutoConfiguration;

#[auto_configuration(after = ["UserDetailsServiceAutoConfiguration"])]
impl WebSecurityAutoConfiguration {
    #[provider( conditional = [Self::on_missing_singleton])]
    fn path_pattern_request_matcher_builder() -> PathPatternRequestMatcherBuilder {
        PathPatternRequestMatcherBuilder::default()
    }

    #[provider(name = "securityFilterChain", order = i32::MAX - 5, conditional = [super::on_default_web_security])]
    fn default_security_filter_chain(
        http_security: &mut HttpSecurity,
    ) -> Arc<dyn SecurityFilterChain> {
        // An authorization the application declared is not extended by the
        // default one, because its `any_request` would be configured after the
        // one of the application.
        if http_security
            .configurer::<AuthorizeHttpRequestsConfigurer<HttpSecurity>>()
            .is_none()
        {
            http_security.authorize_http_requests(|requests| {
                requests.any_request().authenticated();
            });
        }

        http_security.form_login(|_| {}).http_basic(|_| {});

        Arc::new(http_security.build())
    }

    fn on_missing_singleton(ctx: &dyn ApplicationContext) -> bool {
        !ctx.contains_singleton_with_default_name::<PathPatternRequestMatcherBuilder>()
    }
}
