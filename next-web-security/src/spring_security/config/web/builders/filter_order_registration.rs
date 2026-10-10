use std::collections::HashMap;

use next_web_core::{filter::CorsFilter, traits::filter::HttpFilter};

use crate::web::{
    access::{intercept::AuthorizationFilter, ErrorTranslationFilter},
    authentication::{
        logout::LogoutFilter,
        ott::{GenerateOneTimeTokenFilter, OneTimeTokenAuthenticationFilter},
        preauth::x509::X509AuthenticationFilter,
        switchuser::SwitchUserFilter,
        ui::{
            DefaultLoginPageGeneratingFilter, DefaultLogoutPageGeneratingFilter,
            DefaultOneTimeTokenSubmitPageGeneratingFilter, DefaultResourcesFilter,
        },
        www::DigestAuthenticationFilter,
        AnonymousAuthenticationFilter, AuthenticationFilter, BasicAuthenticationFilter,
        RememberMeAuthenticationFilter, UsernamePasswordAuthenticationFilter,
    },
    context::SecurityContextHolderFilter,
    csrf::CsrfFilter,
    header::HeaderWriterFilter,
    savedrequest::RequestCacheAwareFilter,
    session::{ConcurrentSessionFilter, SessionManagementFilter},
    transport::HttpsRedirectFilter,
};

/// An internal use only Comparator that sorts the Security Filter instances to ensure they are in the correct order.
#[derive(Clone)]
pub struct FilterOrderRegistration {
    filter_to_order: HashMap<&'static str, i32>,
}

impl FilterOrderRegistration {
    /// Register a Filter with its specific position. If the Filter was already registered before,
    /// the position previously defined is not going to be overridden
    pub fn put<F: HttpFilter>(&mut self, position: i32) {
        self.filter_to_order
            .entry(std::any::type_name::<F>())
            .or_insert(position);
    }

    /// Returns the order of a particular Filter type by its type.
    pub fn get_order<F: HttpFilter>(&self) -> Option<i32> {
        self.filter_to_order
            .get(std::any::type_name::<F>())
            .map(|v| *v)
    }

    /// Returns the order of a particular Filter type by its name.
    pub fn get_order_by_name(&self, filter_name: &str) -> Option<i32> {
        self.filter_to_order.get(filter_name).copied()
    }
}

impl Default for FilterOrderRegistration {
    fn default() -> Self {
        let step_size = 100;
        let mut order = Step::new(step_size, step_size);
        let mut filter_order = Self {
            filter_to_order: HashMap::with_capacity(30),
        };

        filter_order.put::<HttpsRedirectFilter>(order.next());
        order.next(); // gh-8105
        filter_order.put::<SecurityContextHolderFilter>(order.next());
        filter_order.put::<HeaderWriterFilter>(order.next());
        filter_order.put::<CorsFilter>(order.next());
        filter_order.put::<CsrfFilter>(order.next());
        filter_order.put::<LogoutFilter>(order.next());

        #[cfg(feature = "oauth2-client")]
        filter_order.put::<crate::oauth2::client::web::OAuth2AuthorizationRequestRedirectFilter>(
            order.next(),
        );
        filter_order.put::<GenerateOneTimeTokenFilter>(order.next());
        filter_order.put::<X509AuthenticationFilter>(order.next());

        #[cfg(feature = "oauth2-client")]
        filter_order
            .put::<crate::oauth2::client::web::OAuth2LoginAuthenticationFilter>(order.next());

        filter_order.put::<UsernamePasswordAuthenticationFilter>(order.next());
        filter_order.put::<OneTimeTokenAuthenticationFilter>(order.next());
        order.next(); // gh-8105
        filter_order.put::<DefaultResourcesFilter>(order.next());
        filter_order.put::<DefaultLoginPageGeneratingFilter>(order.next());
        filter_order.put::<DefaultLogoutPageGeneratingFilter>(order.next());
        filter_order.put::<DefaultOneTimeTokenSubmitPageGeneratingFilter>(order.next());
        filter_order.put::<ConcurrentSessionFilter>(order.next());
        filter_order.put::<DigestAuthenticationFilter>(order.next());

        #[cfg(feature = "oauth2-resource-server")]
        filter_order.put::<crate::oauth2::server::resource::web::BearerTokenAuthenticationFilter>(
            order.next(),
        );

        // The endpoints that are used by a client run before the filters that
        // authenticate a resource owner, so that the credentials a client
        // presents are not read as the credentials of a resource owner. The
        // authorization endpoint runs after the error translation filter, so
        // that a resource owner that is not logged in is sent to the login
        // page, and before the authorization filter, so that the endpoint
        // handles its own request.
        #[cfg(feature = "oauth2-authorization-server")]
        {
            use crate::oauth2::server::authorization::{
                AuthorizationEndpointFilter, AuthorizationServerMetadataEndpointFilter,
                JwkSetEndpointFilter, TokenEndpointFilter, TokenIntrospectionEndpointFilter,
                TokenRevocationEndpointFilter,
            };

            filter_order.put::<AuthorizationServerMetadataEndpointFilter>(710);
            filter_order.put::<JwkSetEndpointFilter>(720);
            filter_order.put::<TokenIntrospectionEndpointFilter>(730);
            filter_order.put::<TokenRevocationEndpointFilter>(740);
            filter_order.put::<TokenEndpointFilter>(750);
            filter_order.put::<AuthorizationEndpointFilter>(2810);
        }

        filter_order.put::<BasicAuthenticationFilter>(order.next());
        filter_order.put::<AuthenticationFilter>(order.next());
        filter_order.put::<RequestCacheAwareFilter>(order.next());
        filter_order.put::<RememberMeAuthenticationFilter>(order.next());
        filter_order.put::<AnonymousAuthenticationFilter>(order.next());
        filter_order.put::<SessionManagementFilter>(order.next());
        filter_order.put::<ErrorTranslationFilter>(order.next());
        filter_order.put::<AuthorizationFilter>(order.next());
        filter_order.put::<SwitchUserFilter>(order.next());

        filter_order
    }
}

struct Step {
    value: i32,
    step_size: i32,
}

impl Step {
    fn new(value: i32, step_size: i32) -> Self {
        Self { value, step_size }
    }

    fn next(&mut self) -> i32 {
        let value = self.value;
        self.value += self.step_size;
        value
    }
}
