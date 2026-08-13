use std::collections::HashMap;

use next_web_core::{filter::CorsFilter, traits::filter::HttpFilter};

use crate::web::{
    access::{intercept::AuthorizationFilter, ErrorTranslationFilter},
    authentication::{
        logout::LogoutFilter,
        ott::{GenerateOneTimeTokenFilter, OneTimeTokenAuthenticationFilter},
        switchuser::SwitchUserFilter,
        ui::{
            DefaultLoginPageGeneratingFilter, DefaultLogoutPageGeneratingFilter,
            DefaultOneTimeTokenSubmitPageGeneratingFilter, DefaultResourcesFilter,
        },
        www::DigestAuthenticationFilter,
        AnonymousAuthenticationFilter, AuthenticationFilter, BaseAuthenticationProcessingFilter,
        BasicAuthenticationFilter, OAuth2AuthorizationRequestRedirectFilter,
        OAuth2LoginAuthenticationFilter, RememberMeAuthenticationFilter,
        UsernamePasswordAuthenticationFilter,
    },
    context::SecurityContextHolderFilter,
    csrf::CsrfFilter,
    header::HeaderWriterFilter,
    savedrequest::RequestCacheAwareFilter,
    session::{ConcurrentSessionFilter, SessionManagementFilter},
    transport::HttpsRedirectFilter,
};

#[derive(Clone)]
pub struct FilterOrderRegistration {
    filter_to_order: HashMap<String, i32>,
}

impl FilterOrderRegistration {
    pub fn put<F: HttpFilter>(&mut self, position: i32) {
        self.filter_to_order
            .entry(std::any::type_name::<F>().to_string())
            .or_insert(position);
    }

    pub fn get_order<F: HttpFilter>(&self) -> Option<i32> {
        self.filter_to_order
            .get(std::any::type_name::<F>())
            .map(|v| *v)
    }

    pub fn get_order_by_name(&self, filter_name: &str) -> Option<i32> {
        self.filter_to_order.get(filter_name).copied()
    }
}

impl Default for FilterOrderRegistration {
    fn default() -> Self {
        // Order follows Spring Security's standard filter ordering:
        // https://docs.spring.io/spring-security/reference/servlet/architecture.html
        // Step size of 100 with appropriate gaps for filter insertion.
        let step_size = 100;
        let mut order = Step::new(step_size, step_size);
        let mut filter_order = Self {
            filter_to_order: Default::default(),
        };

        filter_order.put::<HttpsRedirectFilter>(order.next());
        order.next();
        filter_order.put::<SecurityContextHolderFilter>(order.next());
        filter_order.put::<HeaderWriterFilter>(order.next());
        filter_order.put::<CorsFilter>(order.next());
        filter_order.put::<CsrfFilter>(order.next());
        filter_order.put::<LogoutFilter>(order.next());
        filter_order.put::<GenerateOneTimeTokenFilter>(order.next());
        // filter_order.put::<X509AuthenticationFilter>(order.next());
        filter_order.put::<BaseAuthenticationProcessingFilter>(order.next());
        filter_order.put::<UsernamePasswordAuthenticationFilter>(order.next());
        filter_order.put::<OneTimeTokenAuthenticationFilter>(order.next());
        filter_order.put::<OAuth2AuthorizationRequestRedirectFilter>(order.next());
        filter_order.put::<OAuth2LoginAuthenticationFilter>(order.next());
        order.next();
        filter_order.put::<DefaultResourcesFilter>(order.next());
        filter_order.put::<DefaultLoginPageGeneratingFilter>(order.next());
        filter_order.put::<DefaultLogoutPageGeneratingFilter>(order.next());
        filter_order.put::<DefaultOneTimeTokenSubmitPageGeneratingFilter>(order.next());
        filter_order.put::<ConcurrentSessionFilter>(order.next());
        filter_order.put::<DigestAuthenticationFilter>(order.next());
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
