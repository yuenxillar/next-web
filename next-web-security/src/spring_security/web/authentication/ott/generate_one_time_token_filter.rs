use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
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
use tracing::trace;

use crate::{
    authentication::ott::one_time_token_service::OneTimeTokenService,
    web::{
        authentication::ott::{
            generate_one_time_token_request_resolver::{
                DefaultGenerateOneTimeTokenRequestResolver, GenerateOneTimeTokenRequestResolver,
            },
            one_time_token_generation_success_handler::OneTimeTokenGenerationSuccessHandler,
        },
        util::matcher::{PathPatternRequestMatcher, RequestMatcher},
    },
};

/// Global counter used to generate unique per-instance filter keys.
/// This mirrors `OncePerRequestFilter`'s behavior where each instance
/// gets a unique attribute name based on `System.identityHashCode(this)`.
static FILTER_INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Filter that processes a One-Time Token generation request.
///
/// When a request matches the configured `RequestMatcher` (by default a
/// `POST` to `/ott/generate`), this filter:
///
/// 1. Resolves a `GenerateOneTimeTokenRequest` from the HTTP request.
/// 2. Generates a `OneTimeToken` via the `OneTimeTokenService`.
/// 3. Invokes the `OneTimeTokenGenerationSuccessHandler` with the token.
#[derive(Clone)]
pub struct GenerateOneTimeTokenFilter {
    /// Unique per-instance attribute key to ensure this filter runs only once
    /// per request. Mirrors `OncePerRequestFilter.getAlreadyFilteredAttributeName()`.
    already_filtered_attribute_name: String,

    /// The service used to generate one-time tokens.
    token_service: Arc<dyn OneTimeTokenService>,

    /// The handler invoked after a token is successfully generated.
    token_generation_success_handler: Arc<dyn OneTimeTokenGenerationSuccessHandler>,

    /// The request matcher that determines whether this filter should process
    /// the request. Defaults to `POST /ott/generate`.
    request_matcher: Arc<dyn RequestMatcher>,

    /// The resolver that extracts a `GenerateOneTimeTokenRequest` from the request.
    /// Defaults to `DefaultGenerateOneTimeTokenRequestResolver`.
    request_resolver: Arc<dyn GenerateOneTimeTokenRequestResolver>,
}

impl GenerateOneTimeTokenFilter {
    /// The default URL for one-time token generation requests.
    pub const DEFAULT_GENERATE_URL: &'static str = "/ott/generate";

    /// The suffix appended to the already-filtered attribute name, matching
    /// Spring Security's `OncePerRequestFilter.ALREADY_FILTERED_SUFFIX`.
    const ALREADY_FILTERED_SUFFIX: &'static str = ".FILTERED";

    /// Creates a new `GenerateOneTimeTokenFilter`.
    ///
    /// # Parameters
    /// * `token_service` - The service used to generate one-time tokens.
    ///   Cannot be null.
    /// * `token_generation_success_handler` - The handler invoked after a token
    ///   is successfully generated. Cannot be null.
    pub fn new(
        token_service: Arc<dyn OneTimeTokenService>,
        token_generation_success_handler: Arc<dyn OneTimeTokenGenerationSuccessHandler>,
    ) -> Self {
        let instance_id = FILTER_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            already_filtered_attribute_name: format!(
                "{}-{}{}",
                std::any::type_name::<Self>(),
                instance_id,
                Self::ALREADY_FILTERED_SUFFIX
            ),
            token_service,
            token_generation_success_handler,
            request_matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                Some(HttpMethod::POST),
                Self::DEFAULT_GENERATE_URL,
            )),
            request_resolver: Arc::new(DefaultGenerateOneTimeTokenRequestResolver::default()),
        }
    }

    /// Use the given `RequestMatcher` to match the request.
    ///
    /// # Parameters
    /// * `request_matcher` - The request matcher to use. Cannot be null.
    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }

    /// Use the given `GenerateOneTimeTokenRequestResolver` to resolve
    /// `GenerateOneTimeTokenRequest`.
    ///
    /// # Parameters
    /// * `request_resolver` - The resolver to use. Cannot be null.
    ///

    pub fn set_request_resolver(
        &mut self,
        request_resolver: Arc<dyn GenerateOneTimeTokenRequestResolver>,
    ) {
        self.request_resolver = request_resolver;
    }
}

#[async_trait]
impl HttpFilter for GenerateOneTimeTokenFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // Ensure the filter is only applied once per request (OncePerRequestFilter).
        let already_filtered_key = &self.already_filtered_attribute_name;
        if request.get_attribute(already_filtered_key).is_some() {
            return filter_chain.do_filter(request, response).await;
        }
        request.set_attribute(already_filtered_key, AnyValue::Boolean(true));

        // Check if this request matches the configured request matcher.
        if !self.request_matcher.matches(request) {
            if tracing::enabled!(tracing::Level::TRACE) {
                trace!("Did not match request to {:?}", self.request_matcher);
            }
            return filter_chain.do_filter(request, response).await;
        }

        // Resolve the generate request from the HTTP request.
        let Some(generate_request) = self.request_resolver.resolve(request) else {
            // No generate request to process — continue the filter chain.
            return filter_chain.do_filter(request, response).await;
        };

        // Generate the one-time token.
        let ott = self.token_service.generate(generate_request);

        // Notify the success handler.
        self.token_generation_success_handler
            .handle(request, response, ott.as_ref());

        Ok(())
    }
}

impl Named for GenerateOneTimeTokenFilter {
    fn name(&self) -> &str {
        "GenerateOneTimeTokenFilter"
    }
}
