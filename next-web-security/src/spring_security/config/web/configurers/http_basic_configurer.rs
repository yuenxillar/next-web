use std::sync::Arc;

use next_web_core::traits::required::Required;

use crate::{
    authorization::AuthenticationManager,
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        authentication::{
            basic_authentication_entry_point::BasicAuthenticationEntryPoint,
            basic_authentication_filter::BasicAuthenticationFilter,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        AuthenticationEntryPoint,
    },
};

/// Configures HTTP Basic authentication.
///
/// The default realm is `"Realm"`. Customize with `realm_name(...)`.
#[derive(Clone)]
pub struct HttpBasicConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<HttpBasicConfigurer<H>, H>>,
{
    realm_name: String,
    authentication_entry_point: Option<Arc<dyn AuthenticationEntryPoint>>,

    base_http_configurer: BaseHttpConfigurer<HttpBasicConfigurer<H>, H>,
}

impl<H> HttpBasicConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<HttpBasicConfigurer<H>, H>>,
{
    /// Set the HTTP Basic realm name. Default: `"Realm"`.
    pub fn realm_name(mut self, realm_name: &str) -> Self {
        self.realm_name = realm_name.to_string();
        self
    }

    /// Set a custom `AuthenticationEntryPoint`. If not set, a
    /// `BasicAuthenticationEntryPoint` with the configured realm is used.
    pub fn authentication_entry_point(
        mut self,
        entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> Self {
        self.authentication_entry_point = Some(entry_point);
        self
    }

    fn get_authentication_entry_point(&self) -> Arc<dyn AuthenticationEntryPoint> {
        self.authentication_entry_point
            .clone()
            .unwrap_or_else(|| Arc::new(BasicAuthenticationEntryPoint::new(&self.realm_name)))
    }
}

impl<H> Default for HttpBasicConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<HttpBasicConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            realm_name: "Realm".to_string(),
            authentication_entry_point: None,
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<HttpBasicConfigurer<H>, H>> for HttpBasicConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<HttpBasicConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<HttpBasicConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for HttpBasicConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for HttpBasicConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // No provider registration needed for HTTP Basic —
        // it delegates to the AuthenticationManager directly.
    }

    fn configure(&mut self, http: &mut H) {
        // Obtain the AuthenticationManager from shared objects.
        // The AuthenticationManager is set by HttpSecurity.before_configure().
        let auth_manager: Arc<dyn AuthenticationManager> =
            match http.shared_object::<Arc<dyn AuthenticationManager>>() {
                Some(m) => m.clone(),
                None => panic!(
                    "AuthenticationManager is required for HttpBasicConfigurer. \
                 Ensure authentication_manager() has been configured."
                ),
            };

        let entry_point = self.get_authentication_entry_point();
        let filter = BasicAuthenticationFilter::new(auth_manager, entry_point);
        http.add_filter(filter);
    }
}
