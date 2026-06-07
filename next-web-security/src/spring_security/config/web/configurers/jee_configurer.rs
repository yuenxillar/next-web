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

/// Configures J2EE container-based pre-authentication.
///
/// When fully implemented, this will:
/// - Create a `J2eePreAuthenticatedProcessingFilter` that reads the
///   container's `getUserPrincipal()` and `isUserInRole()` methods
/// - Register a `PreAuthenticatedAuthenticationProvider`
/// - Set an `Http403ForbiddenEntryPoint` as the `AuthenticationEntryPoint`
///
/// Usage:
/// ```ignore
/// http.jee(|jee| {
///     jee.mappable_roles(&["USER", "ADMIN"]);
/// });
/// ```
#[derive(Clone)]
pub struct JeeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<JeeConfigurer<H>, H>>,
{
    mappable_roles: Vec<String>,
    base_http_configurer: BaseHttpConfigurer<JeeConfigurer<H>, H>,
}

impl<H> JeeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<JeeConfigurer<H>, H>>,
{
    /// Set the roles to look up from the container.
    /// Each role is automatically prefixed with `"ROLE_"`.
    pub fn mappable_roles(mut self, roles: &[impl AsRef<str>]) -> Self {
        self.mappable_roles = roles
            .iter()
            .map(|r| format!("ROLE_{}", r.as_ref()))
            .collect();
        self
    }

    /// Set the authorities directly (no `ROLE_` prefix added).
    pub fn mappable_authorities(mut self, authorities: &[impl AsRef<str>]) -> Self {
        self.mappable_roles = authorities.iter().map(|a| a.as_ref().to_string()).collect();
        self
    }
}

impl<H> Default for JeeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<JeeConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            mappable_roles: Vec::new(),
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<JeeConfigurer<H>, H>> for JeeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<JeeConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<JeeConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for JeeConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for JeeConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // Stub: Register PreAuthenticatedAuthenticationProvider + Http403ForbiddenEntryPoint.
    }

    fn configure(&mut self, _http: &mut H) {
        // Stub: Create J2eePreAuthenticatedProcessingFilter with mappable roles.
    }
}
