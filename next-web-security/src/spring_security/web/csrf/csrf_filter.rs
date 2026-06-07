use std::sync::{Arc, OnceLock};

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
    util::http_method::HttpMethod,
};

use crate::web::{
    access::{AccessDeniedError, AccessDeniedHandler, AccessDeniedHandlerImpl},
    csrf::{CsrfTokenRepository, CsrfTokenRequestHandler, XorCsrfTokenRequestAttributeHandler},
    util::matcher::RequestMatcher,
};

static DEFAULT_CSRF_MATCHER: OnceLock<Arc<dyn RequestMatcher>> = OnceLock::new();

#[derive(Clone)]
pub struct CsrfFilter {
    token_repository: Arc<dyn CsrfTokenRepository>,

    require_csrf_protection_matcher: Arc<dyn RequestMatcher>,
    access_denied_handler: Arc<dyn AccessDeniedHandler>,
    request_handler: Arc<dyn CsrfTokenRequestHandler>,
}

impl CsrfFilter {
    pub fn new(token_repository: Arc<dyn CsrfTokenRepository>) -> Self {
        let request_handler = Arc::new(XorCsrfTokenRequestAttributeHandler::default());
        let access_denied_handler = Arc::new(AccessDeniedHandlerImpl::default());
        let require_csrf_protection_matcher = Self::default_csrf_matcher();

        Self {
            token_repository,
            require_csrf_protection_matcher,
            access_denied_handler,
            request_handler,
        }
    }

    pub fn set_require_csrf_protection_matcher(
        &mut self,
        require_csrf_protection_matcher: Arc<dyn RequestMatcher>,
    ) {
        self.require_csrf_protection_matcher = require_csrf_protection_matcher;
    }

    pub fn set_request_handler(&mut self, request_handler: Arc<dyn CsrfTokenRequestHandler>) {
        self.request_handler = request_handler;
    }

    pub fn set_access_denied_handler(
        &mut self,
        access_denied_handler: Arc<dyn AccessDeniedHandler>,
    ) {
        self.access_denied_handler = access_denied_handler;
    }
}

impl CsrfFilter {
    pub fn default_csrf_matcher() -> Arc<dyn RequestMatcher> {
        DEFAULT_CSRF_MATCHER
            .get_or_init(|| Arc::new(DefaultRequiresCsrfMatcher::default()))
            .clone()
    }
}

/// Default CSRF protection matcher: requires CSRF for mutating HTTP methods.
#[derive(Debug, Clone, Default)]
struct DefaultRequiresCsrfMatcher {}

impl RequestMatcher for DefaultRequiresCsrfMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        // CSRF protection is required for state-changing methods.
        // Safe methods (GET, HEAD, OPTIONS, TRACE) are excluded.
        !matches!(
            request.method(),
            HttpMethod::Get | HttpMethod::Head | HttpMethod::Options | HttpMethod::Trace
        )
    }
}

#[async_trait]
impl HttpFilter for CsrfFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        // Check if CSRF protection is required for this request
        if !self.require_csrf_protection_matcher.matches(request) {
            return filter_chain.do_filter(request, response).await;
        }

        // Load the expected token from the repository
        let expected_token = self.token_repository.load_token(request).await;

        // Resolve the actual token from the request (header or parameter)
        let actual_token = match &expected_token {
            Some(token) => self
                .request_handler
                .resolve_csrf_token_value(request, token.as_ref()),
            None => None,
        };

        // If no token found or token mismatch → deny access
        let is_valid = match (&expected_token, &actual_token) {
            (Some(expected), Some(actual)) if !actual.is_empty() => {
                expected.get_token() == actual
            }
            _ => false,
        };

        if !is_valid {
            self.access_denied_handler.handle(
                request,
                response,
                AccessDeniedError("Access Denied: Invalid CSRF Token".to_string()),
            )?;
            return Ok(());
        }

        // Token valid — proceed
        filter_chain.do_filter(request, response).await
    }
}

impl Named for CsrfFilter {
    fn name(&self) -> &str {
        "CsrfFilter"
    }
}
