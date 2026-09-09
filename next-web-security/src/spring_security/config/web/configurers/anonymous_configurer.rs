use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::required::Required;

use crate::{
    authentication::{
        anonymous_authentication_provider::AnonymousAuthenticationProvider, AuthenticationProvider,
    },
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::{authority::AuthorityUtils, GrantedAuthority},
    web::{
        authentication::{AnonymousAuthenticationFilter, AuthPrincipal, Identity},
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

use uuid::Uuid;

/// Configures Anonymous authentication (i.e. populate an `Authentication` that
/// represents an anonymous user instead of having a null value) for an
/// `HttpSecurity`. Specifically this will configure an
/// `AnonymousAuthenticationFilter` and an `AnonymousAuthenticationProvider`.
/// All properties have reasonable defaults, so no additional configuration is required
/// other than applying this `SecurityConfigurer`.
#[derive(Clone)]
pub struct AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    key: Option<String>,
    authentication_provider: Option<Arc<dyn AuthenticationProvider>>,
    authentication_filter: Option<AnonymousAuthenticationFilter>,
    principal: AuthPrincipal,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    computed_key: Option<String>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Sets the key to identify tokens created for anonymous authentication.
    /// Default is a secure randomly generated key.
    ///
    /// # Arguments
    ///
    /// * `key` - the key to identify tokens created for anonymous authentication.
    ///
    /// # Returns
    ///
    /// The `AnonymousConfigurer` for further customization of anonymous authentication
    pub fn key(&mut self, key: impl Into<String>) -> &mut Self {
        self.key = Some(key.into());

        self
    }

    /// Sets the principal for `Authentication` objects of anonymous users
    ///
    /// # Arguments
    ///
    /// * `principal` - used for the `Authentication` object of anonymous users
    ///
    /// # Returns
    ///
    /// The `AnonymousConfigurer` for further customization of anonymous authentication
    pub fn principal<T>(&mut self, principal: T) -> &mut Self
    where
        T: Identity,
        T: 'static,
    {
        self.principal = Arc::new(principal);

        self
    }

    /// Sets the authorities for anonymous users
    ///
    /// # Arguments
    ///
    /// * `authorities` - Sets the authorities for anonymous users
    ///
    /// # Returns
    ///
    /// The `AnonymousConfigurer` for further customization of anonymous authentication
    pub fn authorities(&mut self, authorities: Vec<Arc<dyn GrantedAuthority>>) -> &mut Self {
        self.authorities = authorities;

        self
    }

    /// Sets the authorities for anonymous users using string representations
    ///
    /// # Arguments
    ///
    /// * `authorities` - Sets the authorities for anonymous users (i.e. "ROLE_ANONYMOUS")
    ///
    /// # Returns
    ///
    /// The `AnonymousConfigurer` for further customization of anonymous authentication
    pub fn authorities_from_strings(&mut self, authorities: Vec<String>) -> &mut Self {
        self.authorities(AuthorityUtils::create_authority_list(authorities))
    }

    /// Sets the `AuthenticationProvider` used to validate an anonymous user.
    /// If this is set, no attributes on the `AnonymousConfigurer` will be set on the
    /// `AuthenticationProvider`.
    ///
    /// # Arguments
    ///
    /// * `authentication_provider` - the `AuthenticationProvider` used to validate
    ///   an anonymous user. Default is `AnonymousAuthenticationProvider`
    ///
    /// # Returns
    ///
    /// The `AnonymousConfigurer` for further customization of anonymous authentication
    pub fn authentication_provider(
        mut self,
        authentication_provider: Arc<dyn AuthenticationProvider>,
    ) -> Self {
        self.authentication_provider = Some(authentication_provider);

        self
    }

    /// Sets the `AnonymousAuthenticationFilter` used to populate an anonymous user.
    /// If this is set, no attributes on the `AnonymousConfigurer` will be set on the
    /// `AnonymousAuthenticationFilter`.
    ///
    /// # Arguments
    ///
    /// * `authentication_filter` - the `AnonymousAuthenticationFilter` used to
    ///   populate an anonymous user.
    ///
    /// # Returns
    ///
    /// The `AnonymousConfigurer` for further customization of anonymous authentication
    pub fn authentication_filter(
        mut self,
        authentication_filter: AnonymousAuthenticationFilter,
    ) -> Self {
        self.authentication_filter = Some(authentication_filter);

        self
    }

    /// Returns the key to use for anonymous authentication.
    /// Generates a random UUID if no key was explicitly set.
    fn get_key(&mut self) -> String {
        if let Some(computed_key) = self.computed_key.as_ref() {
            return computed_key.clone();
        }

        let key = self
            .key
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        self.computed_key = Some(key.clone());

        key
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, http: &mut H) {
        if self.authentication_provider.is_none() {
            let key = self.get_key();
            self.authentication_provider =
                Some(Arc::new(AnonymousAuthenticationProvider::new(&key)));
        }

        if let Some(provider) = self.authentication_provider.as_ref() {
            http.authentication_provider(provider.to_owned());
        }
    }

    fn configure(&mut self, http: &mut H) {
        let filter = match self.authentication_filter.take() {
            Some(mut filter) => {
                filter.set_security_context_holder_strategy(
                    self.base.get_security_context_holder_strategy().to_owned(),
                );
                filter.after_properties_set();

                filter
            }
            None => AnonymousAuthenticationFilter::new(
                self.get_key(),
                self.principal.clone(),
                self.authorities.clone(),
            ),
        };

        http.add_filter(filter);
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for AnonymousConfigurer<H>
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

impl<H> Deref for AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Default for AnonymousConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            key: None,
            authentication_provider: None,
            authentication_filter: None,
            principal: Arc::new("anonymousUser".to_string()),
            authorities: AuthorityUtils::create_authority_list(["ROLE_ANONYMOUS".to_string()]),
            computed_key: None,

            base: Default::default(),
        }
    }
}
