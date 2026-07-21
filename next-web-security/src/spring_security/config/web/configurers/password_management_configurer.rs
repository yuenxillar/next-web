use std::ops::{Deref, DerefMut};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

/// Configures password management (change password flow).
///
/// When fully implemented, this will add a `RequestMatcherRedirectFilter`
/// redirecting from `/.well-known/change-password` to the configured
/// change password page (default: `/change-password`).
#[derive(Clone)]
pub struct PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    change_password_page: String,
    inner: BaseHttpConfigurer<PasswordManagementConfigurer<H>, H>,
}

impl<H> PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Set the change password page URL. Default: `"/change-password"`.
    pub fn change_password_page(mut self, page: &str) -> Self {
        self.change_password_page = page.to_string();
        self
    }
}

impl<H> Default for PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            change_password_page: "/change-password".to_string(),
            inner: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.inner.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, _http: &mut H) {
        // Stub: Requires RequestMatcherRedirectFilter.
        // When implemented:
        //   let filter = RequestMatcherRedirectFilter::new(
        //       "/.well-known/change-password", &self.change_password_page);
        //   http.add_filter_before::<_, UsernamePasswordAuthenticationFilter>(filter);
    }
}

impl<H> Deref for PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<H> DerefMut for PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
