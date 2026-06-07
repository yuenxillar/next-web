use base64::{
    engine::general_purpose::{self, STANDARD},
    Engine as _,
};
use next_web_core::{
    http::cookie::Cookie,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::WebUtils,
};
use std::sync::Arc;
use tracing::debug;

use crate::web::{
    savedrequest::{DefaultSavedRequest, RequestCache, SavedRequest, SavedRequestAwareWrapper},
    util::{
        matcher::{AnyRequestMatcher, RequestMatcher},
        UrlUtils,
    },
};

const COOKIE_NAME: &str = "REDIRECT_URI";
const COOKIE_MAX_AGE: i32 = -1;

/// An Implementation of `RequestCache` which saves the original request URI in a cookie.
#[derive(Clone)]
pub struct CookieRequestCache {
    request_matcher: Arc<dyn RequestMatcher>,
    cookie_customizer: Arc<dyn Fn(&mut Cookie) + Send + Sync>,
}

impl CookieRequestCache {
    /// Allows selective use of saved requests for a subset of requests.
    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }

    /// Sets the cookie customizer function.
    pub fn set_cookie_customizer<F>(&mut self, cookie_customizer: F)
    where
        F: Fn(&mut Cookie) + Send + Sync + 'static,
    {
        self.cookie_customizer = Arc::new(cookie_customizer);
    }

    fn encode_cookie(cookie_value: &str) -> String {
        STANDARD.encode(cookie_value.as_bytes())
    }

    fn decode_cookie(&self, encoded_cookie_value: &str) -> Option<String> {
        match STANDARD.decode(encoded_cookie_value.as_bytes()) {
            Ok(bytes) => String::from_utf8(bytes).ok(),
            Err(_e) => {
                debug!("Failed decode cookie value {}", encoded_cookie_value);
                None
            }
        }
    }

    fn get_cookie_path(request: &dyn HttpRequest) -> String {
        let context_path = request.context_path().unwrap_or_default();
        if !context_path.is_empty() {
            context_path.to_string()
        } else {
            "/".to_string()
        }
    }

    fn matches_saved_request(
        &self,
        request: &dyn HttpRequest,
        saved_request: Option<&dyn SavedRequest>,
    ) -> bool {
        match saved_request {
            Some(saved) => {
                let current_url = UrlUtils::build_full_request_url(request);
                saved.get_redirect_url() == current_url
            }
            None => false,
        }
    }

    fn get_port(request: &dyn HttpRequest) -> u16 {
        match request.server_port() {
            Some(p) => p,
            None => {
                if request.is_secure() {
                    443
                } else {
                    80
                }
            }
        }
    }
}

impl RequestCache for CookieRequestCache {
    fn save_request(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if !self.request_matcher.matches(request) {
            debug!("Request not saved as configured RequestMatcher did not match");
            return;
        }

        let redirect_url = UrlUtils::build_full_request_url(request);
        let mut saved_cookie = Cookie::new(COOKIE_NAME, Self::encode_cookie(&redirect_url));
        saved_cookie.set_max_age(COOKIE_MAX_AGE);
        saved_cookie.set_secure(request.is_secure());
        saved_cookie.set_path(Self::get_cookie_path(request));
        saved_cookie.set_http_only(true);

        (self.cookie_customizer)(&mut saved_cookie);

        response.add_cookie(saved_cookie);
    }

    fn get_request(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn SavedRequest>> {
        let saved_request_cookie = WebUtils::get_cookie(request, COOKIE_NAME)?;
        let original_uri = self.decode_cookie(saved_request_cookie.value())?;

        let port = Self::get_port(request);

        let mut builder = DefaultSavedRequest::builder();
        let saved = builder
            .set_scheme(request.scheme().map(|s| s.to_string()))
            .set_server_name(request.host().map(|s| s.to_string()))
            .set_request_uri(request.path().to_string())
            .set_query_string(request.query().map(|s| s.to_string()))
            .set_server_port(port)
            .set_method(request.method())
            .set_locales(request.locales())
            .set_parameters(request.parameters())
            .build();

        Some(Arc::new(saved))
    }

    fn get_matching_request(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Box<dyn HttpRequest>> {
        let saved = self.get_request(request, response);

        if !self.matches_saved_request(request, saved.as_ref().map(|s| s.as_ref())) {
            debug!("saved request doesn't match");
            return None;
        }

        self.remove_request(request, response);

        // We know saved is non-null because matches_saved_request returned true
        let saved = match saved {
            Some(s) => s,
            None => return None,
        };
        // Some(Box::new(SavedRequestAwareWrapper::new(saved)))
        //
        todo!()
    }

    fn remove_request(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        let mut remove_saved_request_cookie = Cookie::new(COOKIE_NAME, "");
        remove_saved_request_cookie.set_secure(request.is_secure());
        remove_saved_request_cookie.set_http_only(true);
        remove_saved_request_cookie.set_path(Self::get_cookie_path(request));
        remove_saved_request_cookie.set_max_age(0);
        response.add_cookie(remove_saved_request_cookie);
    }
}

impl Default for CookieRequestCache {
    fn default() -> Self {
        Self {
            request_matcher: AnyRequestMatcher::instance(),
            cookie_customizer: Arc::new(|_| {}),
        }
    }
}
