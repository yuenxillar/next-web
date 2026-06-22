use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{
    core::Authentication,
    web::authentication::{
        authentication_success_handler::AuthenticationSuccessHandler,
        BaseAuthenticationTargetUrlRequestHandler,
    },
};

/// A simple `AuthenticationSuccessHandler` that redirects to the given URL
/// on successful authentication.
///
/// This is the equivalent of Spring Security's `SimpleUrlAuthenticationSuccessHandler`.
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
        base.set_always_use_default_target_url(true);
        Self { base }
    }

    /// Sets the default target URL.
    pub fn set_default_target_url(&mut self, default_target_url: impl Into<Box<str>>) {
        self.base.set_default_target_url(default_target_url);
    }

    /// Sets whether to always use the default target URL.
    pub fn set_always_use_default_target_url(&mut self, always_use: bool) {
        self.base.set_always_use_default_target_url(always_use);
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

impl AuthenticationSuccessHandler for SimpleUrlAuthenticationSuccessHandler {
    fn on_authentication_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: &dyn Authentication,
    ) {
        let _ = self.base.handle(request, response, Some(authentication));
    }
}
