use std::{
    any::type_name,
    fmt::Display,
    sync::{Arc, OnceLock},
};

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    filter::FilterError,
    http::HttpMethod,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::{debug, trace};
use tracing::{enabled, Level};

use crate::{
    access::AccessDeniedError,
    web::{
        access::{AccessDeniedHandler, AccessDeniedHandlerImpl},
        csrf::{
            csrf_token_repository::load_deferred_token, CsrfTokenRepository,
            CsrfTokenRequestHandler, DeferredCsrfToken, XorCsrfTokenRequestAttributeHandler,
        },
        util::{matcher::RequestMatcher, UrlUtils},
    },
};

/// The default RequestMatcher that indicates if CSRF protection is required or not.
/// The default is to ignore GET, HEAD, TRACE, OPTIONS and process all other requests.
static DEFAULT_CSRF_MATCHER: OnceLock<Arc<dyn RequestMatcher>> = OnceLock::new();

/// Applies CSRF   protection using a synchronizer token pattern. Developers are required to ensure that
/// CsrfFilter is invoked for any request that allows state to change. Typically this just means that they
/// should ensure their web application follows proper REST semantics (i.e. do not change state with the HTTP methods GET, HEAD, TRACE, OPTIONS).
///
/// Typically the CsrfTokenRepository implementation chooses to store the CsrfToken in HttpSession with HttpSessionCsrfTokenRepository.
/// This is preferred to storing the token in a cookie which can be modified by a client application.
#[derive(Clone)]
pub struct CsrfFilter {
    token_repository: Arc<dyn CsrfTokenRepository>,
    require_csrf_protection_matcher: Arc<dyn RequestMatcher>,
    access_denied_handler: Arc<dyn AccessDeniedHandler>,
    request_handler: Arc<dyn CsrfTokenRequestHandler>,
}

impl CsrfFilter {
    /// Creates a new instance.
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

    /// Specifies a RequestMatcher that is used to determine if CSRF protection should be applied. If the
    /// RequestMatcher returns true for a given request, then CSRF protection is applied.
    /// The default is to apply CSRF protection for any HTTP method other than GET, HEAD, TRACE, OPTIONS.
    pub fn set_require_csrf_protection_matcher(
        &mut self,
        require_csrf_protection_matcher: Arc<dyn RequestMatcher>,
    ) {
        self.require_csrf_protection_matcher = require_csrf_protection_matcher;
    }

    /// Specifies a CsrfTokenRequestHandler that is used to make the CsrfToken available as a request attribute.
    /// The default is XorCsrfTokenRequestAttributeHandler.
    pub fn set_request_handler(&mut self, request_handler: Arc<dyn CsrfTokenRequestHandler>) {
        self.request_handler = request_handler;
    }

    /// Specifies a AccessDeniedHandler that should be used when CSRF protection fails.
    /// The default is to use AccessDeniedHandlerImpl with no arguments.
    pub fn set_access_denied_handler(
        &mut self,
        access_denied_handler: Arc<dyn AccessDeniedHandler>,
    ) {
        self.access_denied_handler = access_denied_handler;
    }

    /// The default RequestMatcher that indicates if CSRF protection is required or not.
    /// The default is to ignore GET, HEAD, TRACE, OPTIONS and process all other requests.
    pub fn default_csrf_matcher() -> Arc<dyn RequestMatcher> {
        DEFAULT_CSRF_MATCHER
            .get_or_init(|| Arc::new(DefaultRequiresCsrfMatcher::default()))
            .clone()
    }

    /// Constant time comparison to prevent against timing attacks.
    fn equals_constant_time(expected: &str, actual: Option<&str>) -> bool {
        match actual {
            None => false,
            Some(actual_str) => {
                if expected.eq(actual_str) {
                    return true;
                }

                let expected_bytes = expected.as_bytes();
                let actual_bytes = actual_str.as_bytes();

                if expected_bytes.len() != actual_bytes.len() {
                    return false;
                }

                let mut result: u8 = 0;
                for (a, b) in expected_bytes.iter().zip(actual_bytes.iter()) {
                    result |= a ^ b;
                }

                result == 0
            }
        }
    }
}

#[async_trait]
impl HttpFilter for CsrfFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let mut deferred_csrf_token =
            load_deferred_token(self.token_repository.clone(), request, response);
        request.set_attribute(
            type_name::<&dyn DeferredCsrfToken>(),
            AnyValue::Object(Box::new(deferred_csrf_token.clone())),
        );
        self.request_handler
            .handle(request, response, &mut deferred_csrf_token)
            .await;

        if !self.require_csrf_protection_matcher.matches(request) {
            if enabled!(Level::TRACE) {
                trace!(
                    "Did not protect against CSRF since request did not match {:?}",
                    self.require_csrf_protection_matcher
                );
            }

            return filter_chain.do_filter(request, response).await;
        }

        let csrf_token = deferred_csrf_token.token().await;
        let actual_token = self
            .request_handler
            .resolve_csrf_token_value(request, csrf_token.as_ref());
        if actual_token.as_ref().filter(|s| !s.is_empty()).is_some() && enabled!(Level::TRACE) {
            trace!("Found a CSRF token in the request");
        }

        if !Self::equals_constant_time(csrf_token.token(), actual_token.as_deref()) {
            let missing_token = deferred_csrf_token.is_generated().await;
            debug!(
                "Invalid CSRF token found for {}",
                UrlUtils::build_full_request_url(request)
            );

            let error = if !missing_token {
                AccessDeniedError::InvalidCsrfToken {
                    expected_access_token: (
                        csrf_token.parameter_name().to_string(),
                        csrf_token.header_name().to_string(),
                    ),
                    actual_access_token: actual_token.clone(),
                }
            } else {
                AccessDeniedError::MissingCsrfToken
            };
            self.access_denied_handler
                .handle(request, response, &error)?;

            return Ok(());
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for CsrfFilter {
    fn name(&self) -> &str {
        "CsrfFilter"
    }
}

/// Default CSRF protection matcher: requires CSRF for mutating HTTP methods.
#[derive(Debug, Clone, Default)]
struct DefaultRequiresCsrfMatcher;

impl RequestMatcher for DefaultRequiresCsrfMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        // CSRF protection is required for state-changing methods.
        // Safe methods (GET, HEAD, OPTIONS, TRACE) are excluded.
        !matches!(
            request.method(),
            HttpMethod::GET | HttpMethod::HEAD | HttpMethod::TRACE | HttpMethod::OPTIONS
        )
    }
}

impl Display for DefaultRequiresCsrfMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IsNotHttpMethod [GET, HEAD, TRACE, OPTIONS]",)
    }
}
