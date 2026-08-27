use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{traits::required::Required, util::StringUtils};

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::{
        authentication::UsernamePasswordAuthenticationFilter,
        default_security_filter_chain::DefaultSecurityFilterChain, util::matcher::DEFAULT_BUILDER,
        RequestMatcherRedirectFilter,
    },
};

/// Adds password management support.
#[derive(Clone)]
pub struct PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    change_password_page: String,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// ets the change password page. Defaults to DEFAULT_CHANGE_PASSWORD_PAGE..
    pub fn change_password_page(&mut self, change_password_page: impl Into<String>) -> &mut Self {
        let change_password_page = change_password_page.into();
        assert!(
            StringUtils::has_text(&change_password_page),
            "change_password_page cannot be empty"
        );
        self.change_password_page = change_password_page;
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
            base: Default::default(),
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
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    /// Configure the SecurityBuilder by setting the necessary properties on the SecurityBuilder.
    fn configure(&mut self, http: &mut H) {
        // Stub: Requires RequestMatcherRedirectFilter.
        // When implemented:
        let filter = RequestMatcherRedirectFilter::new(
            Arc::new(
                DEFAULT_BUILDER
                    .get_or_init(|| Default::default())
                    .matcher(None, "/.well-known/change-password"),
            ),
            &self.change_password_page,
        );
        http.add_filter_before::<_, UsernamePasswordAuthenticationFilter>(filter);
    }
}

impl<H> Deref for PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for PasswordManagementConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
