use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    cors::{CorsConfigurationSource, PreFlightRequestHandler},
    filter::{CorsFilter, PreFlightRequestFilter},
    traits::required::Required,
    ApplicationContext,
};

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

/// Bean name of the `CorsConfigurationSource` singleton.
const CORS_CONFIGURATION_SOURCE_BEAN_NAME: &str = "corsConfigurationSource";
/// Bean name of the `CorsFilter` singleton.
const CORS_FILTER_BEAN_NAME: &str = "corsFilter";

/// Configures CORS (Cross-Origin Resource Sharing) support.
///
/// Adds `CorsFilter` or `PreFlightRequestFilter` to the security filter chain.
/// If a singleton named `corsFilter` is provided, that `CorsFilter` is used.
/// Otherwise, if `corsConfigurationSource` is defined, that
/// `CorsConfigurationSource` is used. If a `PreFlightRequestHandler` is set on
/// this configurer, `CorsFilter` is not used and `PreFlightRequestFilter` is
/// registered instead.
#[derive(Clone)]
pub struct CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    configuration_source: Option<Arc<dyn CorsConfigurationSource>>,
    pre_flight_request_handler: Option<Arc<dyn PreFlightRequestHandler>>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Sets the `CorsConfigurationSource` used by the `CorsFilter`.
    pub fn configuration_source(
        &mut self,
        configuration_source: Arc<dyn CorsConfigurationSource>,
    ) -> &mut Self {
        self.configuration_source = Some(configuration_source);
        self
    }

    /// Use the given `PreFlightRequestHandler` for CORS pre-flight requests.
    /// When set, `CorsFilter` is not used. Cannot be combined with
    /// `configuration_source`.
    pub fn pre_flight_request_handler(
        &mut self,
        pre_flight_request_handler: Arc<dyn PreFlightRequestHandler>,
    ) -> &mut Self {
        self.pre_flight_request_handler = Some(pre_flight_request_handler);
        self
    }

    fn get_pre_flight_request_handler(&self, http: &H) -> Option<Arc<dyn PreFlightRequestHandler>> {
        if self.configuration_source.is_some() {
            return None;
        }
        if let Some(handler) = &self.pre_flight_request_handler {
            return Some(handler.clone());
        }
        http.shared_object::<ApplicationContext>()?
            .get_single_option::<Arc<dyn PreFlightRequestHandler>>()
            .map(Clone::clone)
    }

    fn get_cors_configuration_source(
        &self,
        context: Option<&ApplicationContext>,
    ) -> Option<Arc<dyn CorsConfigurationSource>> {
        context
            .as_ref()
            .and_then(|ctx| {
                ctx.get_single_option_with_name::<Arc<dyn CorsConfigurationSource>>(
                    CORS_CONFIGURATION_SOURCE_BEAN_NAME,
                )
            })
            .map(Clone::clone)
    }

    fn get_cors_filter(&mut self, http: &H) -> Option<CorsFilter> {
        if self.pre_flight_request_handler.is_some() {
            return None;
        }

        if let Some(source) = self.configuration_source.take() {
            return Some(CorsFilter::new(source));
        }
        let context = http.shared_object::<ApplicationContext>();
        if let Some(filter) = context
            .and_then(|ctx| ctx.get_single_option_with_name::<CorsFilter>(CORS_FILTER_BEAN_NAME))
        {
            return Some(filter.clone());
        }
        self.get_cors_configuration_source(context)
            .map(CorsFilter::new)
    }
}

impl<H> Deref for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        if self.configuration_source.is_some() && self.pre_flight_request_handler.is_some() {
            panic!(
                "Cannot configure both a CorsConfigurationSource and a PreFlightRequestHandler on CorsConfigurer"
            );
        }

        if let Some(cors_filter) = self.get_cors_filter(http) {
            http.add_filter(cors_filter);
            return;
        }

        if let Some(handler) = self.get_pre_flight_request_handler(http) {
            let filter = PreFlightRequestFilter::new(handler);
            http.add_filter_before::<PreFlightRequestFilter, CorsFilter>(filter);
            return;
        }

        panic!(
            "Failed to find a bean that implements `CorsConfigurationSource`. Please ensure that you are using `@EnableWebMvc`, are publishing a `WebMvcConfigurer`, or are publishing a `CorsConfigurationSource` bean."
        );
    }
}

impl<H> Default for CorsConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            configuration_source: None,
            pre_flight_request_handler: None,
            base: Default::default(),
        }
    }
}
