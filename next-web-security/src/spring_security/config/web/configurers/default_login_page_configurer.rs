//! Adds the filters that generate the default login and logout pages.

use std::{collections::HashMap, sync::Arc};

use next_web_core::traits::http::http_request::HttpRequest;

use crate::{
    config::{
        security_configurer::SecurityConfigurer,
        web::{
            configurers::{BaseHttpConfigurer, ErrorHandlingConfigurer, LogoutConfigurer},
            HttpSecurityBuilder,
        },
    },
    web::{
        authentication::ui::{
            DefaultLoginPageGeneratingFilter, DefaultLogoutPageGeneratingFilter,
            DefaultResourcesFilter,
        },
        csrf::CsrfToken,
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

/// Adds the filter that generates a login page when the application did not
/// contribute one.
///
/// The page is generated only when a login method is enabled and the
/// application did not contribute an authentication entry point of its own,
/// because an entry point decides where an unauthenticated request is sent.
///
/// # Shared objects created
///
/// The configurer contributes the [`DefaultLoginPageGeneratingFilter`] it
/// configures as a shared object, so that the configurations of the login
/// methods can describe the page they need: the form login configuration
/// enables the form of the page and sets the URLs it posts to, and the other
/// configurations add the sections they need.
///
/// # Shared objects used
///
/// No shared object is used. The filter is configured from the [[`FormLoginConfigurer`]]
/// through the shared object the configurer contributes.
///
/// [`FormLoginConfigurer`]: crate::config::web::configurers::FormLoginConfigurer
#[derive(Clone)]
pub struct DefaultLoginPageConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// The filter that generates the login page.
    login_page_generating_filter: DefaultLoginPageGeneratingFilter,
    /// The filter that generates the page confirming a logout.
    logout_page_generating_filter: DefaultLogoutPageGeneratingFilter,

    /// The configurer adapter the configurer shares with the other
    /// configurations of the HTTP security.
    base: BaseHttpConfigurer<Self, H>,
}

impl<H> DefaultLoginPageConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Returns the hidden inputs the generated forms carry.
    ///
    /// The inputs are the CSRF token of the request, so that a form the default
    /// page renders is accepted when CSRF protection is enabled. The token is
    /// the request attribute the CSRF filter writes.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the token is read from.
    fn hidden_inputs(request: &dyn HttpRequest) -> HashMap<String, String> {
        let token = request
            .get_attribute(std::any::type_name::<&dyn CsrfToken>())
            .and_then(|value| value.as_ref_value::<Arc<dyn CsrfToken>>());

        match token {
            Some(token) => HashMap::from([(
                token.parameter_name().to_string(),
                token.token().to_string(),
            )]),
            None => HashMap::new(),
        }
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for DefaultLoginPageConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    /// Prepares the filters the configurer contributes.
    ///
    /// The login page filter is registered as a shared object, so that the
    /// configurations of the login methods can enable the parts of the page
    /// before the filters are added.
    fn init(&mut self, http: &mut H) {
        self.login_page_generating_filter
            .set_security_context_holder_strategy(
                self.base.get_security_context_holder_strategy().clone(),
            );
        self.login_page_generating_filter
            .set_resolve_hidden_inputs(Self::hidden_inputs);
        self.logout_page_generating_filter
            .set_resolve_hidden_inputs(Arc::new(Self::hidden_inputs));

        http.set_shared_object(self.login_page_generating_filter.clone());
    }

    /// Adds the filters of the default pages.
    ///
    /// The filters are added only when a login method is enabled and the
    /// application did not contribute an authentication entry point, because
    /// the entry point decides where an unauthenticated request is sent.
    fn configure(&mut self, http: &mut H) {
        // The configurations of the login methods describe the page through the
        // shared object the configurer contributes, so the page is read back
        // from it before it is added.
        if let Some(login_page_generating_filter) = http
            .shared_object::<DefaultLoginPageGeneratingFilter>()
            .cloned()
        {
            self.login_page_generating_filter = login_page_generating_filter;
        }

        let authentication_entry_point = http
            .configurer::<ErrorHandlingConfigurer<H>>()
            .and_then(|configurer| configurer.get_authentication_entry_point())
            .cloned();

        if !self.login_page_generating_filter.is_enabled() || authentication_entry_point.is_some() {
            return;
        }

        http.add_filter(self.login_page_generating_filter.clone());
        http.add_filter(DefaultResourcesFilter::css());

        if http.configurer::<LogoutConfigurer<H>>().is_some() {
            http.add_filter(self.logout_page_generating_filter.clone());
        }
    }
}

impl<H> Default for DefaultLoginPageConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn default() -> Self {
        Self {
            login_page_generating_filter: DefaultLoginPageGeneratingFilter::default(),
            logout_page_generating_filter: DefaultLogoutPageGeneratingFilter::default(),

            base: BaseHttpConfigurer::default(),
        }
    }
}
