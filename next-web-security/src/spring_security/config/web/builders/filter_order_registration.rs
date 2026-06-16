use std::collections::HashMap;

use next_web_core::traits::filter::HttpFilter;

use crate::web::{
    access::{intercept::AuthorizationFilter, ErrorTranslationFilter},
    authentication::https_redirect_filter::HttpsRedirectFilter,
    authentication::{
        anonymous_authentication_filter::AnonymousAuthenticationFilter,
        basic_authentication_filter::BasicAuthenticationFilter, logout::LogoutFilter,
        remember_me_authentication_filter::RememberMeAuthenticationFilter,
        ui::default_login_page_generating_filter::DefaultLoginPageGeneratingFilter,
        username_password_authentication_filter::UsernamePasswordAuthenticationFilter,
    },
    csrf::CsrfFilter,
    header::HeaderWriterFilter,
    savedrequest::RequestCacheAwareFilter,
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

        // HttpsRedirectFilter — redirects HTTP to HTTPS
        filter_order.put::<HttpsRedirectFilter>(order.next());
        order.next();

        // LogoutFilter — handles /logout
        filter_order.put::<LogoutFilter>(order.next());
        order.next();

        // CsrfFilter — validates CSRF tokens for mutating requests
        filter_order.put::<CsrfFilter>(order.next());
        order.next();

        // UsernamePasswordAuthenticationFilter — processes form login
        filter_order.put::<UsernamePasswordAuthenticationFilter>(order.next());
        order.next();

        // HeaderWriterFilter — writes security headers to the response
        filter_order.put::<HeaderWriterFilter>(order.next());
        order.next();

        // BasicAuthenticationFilter — processes HTTP Basic auth
        filter_order.put::<BasicAuthenticationFilter>(order.next());
        order.next();

        // DefaultLoginPageGeneratingFilter — auto-generated login page
        filter_order.put::<DefaultLoginPageGeneratingFilter>(order.next());
        order.next();

        // ErrorTranslationFilter — translates auth exceptions to HTTP responses
        filter_order.put::<ErrorTranslationFilter>(order.next());
        order.next();

        // RequestCacheAwareFilter — replays saved requests after auth
        filter_order.put::<RequestCacheAwareFilter>(order.next());
        order.next();

        // RememberMeAuthenticationFilter — auto-login from persistent cookie
        filter_order.put::<RememberMeAuthenticationFilter>(order.next());
        order.next();

        // AnonymousAuthenticationFilter — populates anonymous SecurityContext
        filter_order.put::<AnonymousAuthenticationFilter>(order.next());
        order.next();

        // AuthorizationFilter — enforces access control rules
        filter_order.put::<AuthorizationFilter>(order.next());

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
