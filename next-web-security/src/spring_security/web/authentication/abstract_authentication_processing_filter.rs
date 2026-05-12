use std::sync::Arc;

use crate::{
    config::web::util::matcher::request_matcher::RequestMatcher,
    web::authentication::{
        remember_me_services::RememberMeServices,
        rememberme::abstract_remember_me_services::AbstractRememberMeServices,
    },
};

#[derive(Clone)]
pub struct AbstractAuthenticationProcessingFilter {
    remember_me_services: Arc<dyn RememberMeServices>,
    requires_authentication_request_matcher: Option<Box<dyn RequestMatcher>>,
}

impl Default for AbstractAuthenticationProcessingFilter {
    fn default() -> Self {
        Self {
            remember_me_services: Arc::new(AbstractRememberMeServices {}),
            requires_authentication_request_matcher: None,
        }
    }
}

impl AbstractAuthenticationProcessingFilter {
    pub fn get_remember_me_services(&self) -> &dyn RememberMeServices {
        self.remember_me_services.as_ref()
    }
}

impl AbstractAuthenticationProcessingFilter {
    pub fn set_requires_authentication_request_matcher(
        &mut self,
        request_matcher: impl RequestMatcher + 'static,
    ) {
        self.requires_authentication_request_matcher = Some(Box::new(request_matcher));
    }
}
