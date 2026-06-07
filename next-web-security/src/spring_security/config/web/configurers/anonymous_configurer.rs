use next_web_core::traits::required::Required;

use crate::{
    authentication::anonymous_authentication_provider::AnonymousAuthenticationProvider,
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
        authentication::anonymous_authentication_filter::AnonymousAuthenticationFilter,
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

/// Configures anonymous authentication. Defaults to `anonymousUser` principal
/// with `ROLE_ANONYMOUS` authority. Activated by default with `@EnableWebSecurity`.
///
/// Use `anonymous.disable()` to represent anonymous users as `null` instead.
#[derive(Clone)]
pub struct AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<AnonymousConfigurer<H>, H>>,
{
    key: Option<String>,
    principal: String,
    authorities: Vec<String>,

    base_http_configurer: BaseHttpConfigurer<AnonymousConfigurer<H>, H>,
}

impl<H> AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<AnonymousConfigurer<H>, H>>,
{
    /// Set the secret key used for anonymous authentication tokens.
    /// If not set, a randomly generated UUID is used.
    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(key.to_string());
        self
    }

    /// Set the principal name for anonymous users. Default: `"anonymousUser"`.
    pub fn principal(mut self, principal: &str) -> Self {
        self.principal = principal.to_string();
        self
    }

    /// Set the authorities for anonymous users. Default: `["ROLE_ANONYMOUS"]`.
    pub fn authorities(mut self, authorities: Vec<String>) -> Self {
        self.authorities = authorities;
        self
    }

    fn get_key(&self) -> String {
        self.key
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }

    fn get_principal(&self) -> String {
        self.principal.clone()
    }

    fn get_authorities(&self) -> Vec<String> {
        self.authorities.clone()
    }
}

impl<H> Default for AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<AnonymousConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            key: None,
            principal: "anonymousUser".to_string(),
            authorities: vec!["ROLE_ANONYMOUS".to_string()],
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<AnonymousConfigurer<H>, H>> for AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<AnonymousConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<AnonymousConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for AnonymousConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, http: &mut H) {
        // Register the AnonymousAuthenticationProvider
        let provider = AnonymousAuthenticationProvider::new(&self.get_key());
        http.authentication_provider(provider);
    }

    fn configure(&mut self, http: &mut H) {
        let filter = AnonymousAuthenticationFilter::new(
            self.get_key(),
            self.get_principal(),
            self.get_authorities(),
        );
        http.add_filter(filter);
    }
}
