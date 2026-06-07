use std::sync::Arc;

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

/// Configures CORS (Cross-Origin Resource Sharing) support.
///
/// When fully implemented, this configurer will create a `CorsFilter`
/// based on a `CorsConfigurationSource` bean or explicit configuration.
#[derive(Clone)]
pub struct CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<CorsConfigurer<H>, H>>,
{
    base_http_configurer: BaseHttpConfigurer<CorsConfigurer<H>, H>,
}

impl<H> Default for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<CorsConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<CorsConfigurer<H>, H>> for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<CorsConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<CorsConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_object()
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        // Stub: Full implementation requires CorsFilter and CorsConfigurationSource.
        // When implemented:
        //   let cors_filter = get_cors_filter(http);
        //   http.add_filter(cors_filter);
        let _ = http; // suppress unused warning
    }
}
