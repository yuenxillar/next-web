use std::ops::{Deref, DerefMut};

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{
    core::Authentication,
    web::{
        authentication::{
            authentication_success_handler::AuthenticationSuccessHandler,
            BaseAuthenticationTargetUrlRequestHandler,
        },
        WebAttributes,
    },
};

/// AuthenticationSuccessHandler which can be configured with a default URL which users should be sent to upon successful authentication.
#[derive(Clone)]
pub struct SimpleUrlAuthenticationSuccessHandler {
    base: BaseAuthenticationTargetUrlRequestHandler,
}

impl SimpleUrlAuthenticationSuccessHandler {
    /// Creates a new `SimpleUrlAuthenticationSuccessHandler` with the given
    /// default target URL.
    pub fn new(default_target_url: &str) -> Self {
        let mut base = BaseAuthenticationTargetUrlRequestHandler::default();
        base.set_default_target_url(default_target_url);

        Self { base }
    }

    /// Removes temporary authentication-related data which may have been stored in the session during the authentication process.
    pub fn clear_authentication_attributes(&self, request: &mut dyn HttpRequest) {
        request.session().map(|session| {
            session.remove_attribute(WebAttributes::AUTHENTICATION_ERROR);
        });
    }
}

impl AuthenticationSuccessHandler for SimpleUrlAuthenticationSuccessHandler {
    /// Calls the parent class handle() method to forward or redirect to the target URL, and then calls
    /// clearAuthenticationAttributes() to remove any leftover session data.
    fn on_authentication_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: &dyn Authentication,
    ) {
        self.base.handle(request, response, Some(authentication));
        self.clear_authentication_attributes(request);
    }
}

impl Deref for SimpleUrlAuthenticationSuccessHandler {
    type Target = BaseAuthenticationTargetUrlRequestHandler;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for SimpleUrlAuthenticationSuccessHandler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Default for SimpleUrlAuthenticationSuccessHandler {
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}
