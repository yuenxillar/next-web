use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::StringUtils,
};

use crate::{
    core::Authentication,
    web::{
        authentication::{
            authentication_success_handler::AuthenticationSuccessHandler,
            SimpleUrlAuthenticationSuccessHandler,
        },
        savedrequest::{HttpSessionRequestCache, RequestCache},
    },
};

/// An authentication success strategy which can make use of the `DefaultSavedRequest` which may
/// have been stored in the session by the `ExceptionTranslationFilter`. When such a
/// request is intercepted and requires authentication, the request data is stored to
/// record the original destination before the authentication process commenced, and to
/// allow the request to be reconstructed when a redirect to the same URL occurs. This
/// class is responsible for performing the redirect to the original URL if appropriate.
///
/// Following a successful authentication, it decides on the redirect destination, based on
/// the following scenarios:
///
/// * If the `always_use_default_target_url` property is set to true, the
///   `default_target_url` will be used for the destination. Any
///   `DefaultSavedRequest` stored in the session will be removed.
/// * If the `target_url_parameter` has been set on the request, the value will be
///   used as the destination. Any `DefaultSavedRequest` will again be removed.
/// * If a `SavedRequest` is found in the `RequestCache` (as set by the
///   `ExceptionTranslationFilter` to record the original destination before the
///   authentication process commenced), a redirect will be performed to the Url of that
///   original destination. The `SavedRequest` object will remain cached and be picked up
///   when the redirected request is received (See `SavedRequestAwareWrapper`).
/// * If no `SavedRequest` is found, it will delegate to the base class.
#[derive(Clone)]
pub struct SavedRequestAwareAuthenticationSuccessHandler {
    request_cache: Arc<dyn RequestCache>,

    inner: SimpleUrlAuthenticationSuccessHandler,
}

impl Deref for SavedRequestAwareAuthenticationSuccessHandler {
    type Target = SimpleUrlAuthenticationSuccessHandler;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SavedRequestAwareAuthenticationSuccessHandler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl SavedRequestAwareAuthenticationSuccessHandler {
    pub fn set_request_cache(&mut self, request_cache: Arc<dyn RequestCache>) {
        self.request_cache = request_cache;
    }
}

impl AuthenticationSuccessHandler for SavedRequestAwareAuthenticationSuccessHandler {
    fn on_authentication_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: &dyn Authentication,
    ) {
        if let Some(save_request) = self.request_cache.get_request(request, response) {
            let target_url_parameter = self.get_target_url_parameter();

            if self.is_always_use_default_target_url()
                || target_url_parameter
                    .map(|s| {
                        request
                            .parameter(s)
                            .map(StringUtils::has_length)
                            .unwrap_or_default()
                    })
                    .unwrap_or_default()
            {
                self.request_cache.remove_request(request, response);
                self.inner
                    .on_authentication_success(request, response, authentication);
                return;
            }

            self.clear_authentication_attributes(request);
            // Use the DefaultSavedRequest URL
            let target_url = save_request.get_redirect_url();
            self.get_redirect_strategy()
                .send_redirect(request, response, &target_url);
        } else {
            self.inner
                .on_authentication_success(request, response, authentication);
        }
    }
}

impl Default for SavedRequestAwareAuthenticationSuccessHandler {
    fn default() -> Self {
        Self {
            request_cache: Arc::new(HttpSessionRequestCache::default()),

            inner: Default::default(),
        }
    }
}
