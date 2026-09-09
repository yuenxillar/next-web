use std::{fmt, ops::Deref, ops::DerefMut, sync::Arc};

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    authorization::AuthenticationManager,
    core::context::SecurityContextHolderStrategy,
    oauth2_resource_server::{
        bearer::{BearerTokenAuthenticationConverter, BearerTokenResolver},
        entry::{AuthenticationEntryPointFailureHandler, BearerTokenAuthenticationEntryPoint},
    },
    web::{
        authentication::{
            BaseAuthenticationProcessingFilter, BaseAuthenticationProcessingFilterExt,
        },
        context::SecurityContextRepository,
        util::matcher::RequestMatcher,
    },
};

/// A `RequestMatcher` that always matches, used so that the
/// `BearerTokenAuthenticationFilter` attempts authentication on every request
/// (the converter simply returns `None` when no bearer token is present).
#[derive(Clone, Debug, Default)]
struct AllRequestMatcher;

impl RequestMatcher for AllRequestMatcher {
    fn matches(&self, _request: &dyn HttpRequest) -> bool {
        true
    }
}

/// The filter that authenticates a bearer token presented to a resource server.
///
/// It wraps `BaseAuthenticationProcessingFilter`, using a
/// `BearerTokenAuthenticationConverter` to resolve the bearer token and the
/// shared `AuthenticationManager` (populated with the JWT or opaque token
/// provider) to authenticate it. On failure it delegates to the configured
/// `AuthenticationEntryPoint` via an `AuthenticationEntryPointFailureHandler`.
#[derive(Clone)]
pub struct BearerTokenAuthenticationFilter {
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    authentication_entry_point: Arc<dyn crate::web::AuthenticationEntryPoint>,
    base: BaseAuthenticationProcessingFilter,
}

impl BearerTokenAuthenticationFilter {
    pub fn new(
        authentication_manager: Arc<dyn AuthenticationManager>,
        bearer_token_resolver: Arc<dyn BearerTokenResolver>,
    ) -> Self {
        let converter = Arc::new(BearerTokenAuthenticationConverter::new(
            bearer_token_resolver,
        ));
        let entry_point: Arc<dyn crate::web::AuthenticationEntryPoint> =
            Arc::new(BearerTokenAuthenticationEntryPoint::default());
        let mut inner =
            BaseAuthenticationProcessingFilter::with_request_matcher(Arc::new(AllRequestMatcher));
        inner.set_authentication_manager(authentication_manager.clone());
        inner.set_authentication_converter(converter);
        inner.set_continue_chain_before_successful_authentication(true);
        inner.set_authentication_failure_handler(Arc::new(
            AuthenticationEntryPointFailureHandler::new(entry_point.clone()),
        ));
        Self {
            authentication_manager: Some(authentication_manager),
            authentication_entry_point: entry_point,
            base: inner,
        }
    }

    pub fn set_authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) {
        self.authentication_manager = Some(authentication_manager.clone());
        self.base.set_authentication_manager(authentication_manager);
    }

    pub fn set_authentication_entry_point(
        &mut self,
        authentication_entry_point: Arc<dyn crate::web::AuthenticationEntryPoint>,
    ) {
        self.authentication_entry_point = authentication_entry_point.clone();
        self.base.set_authentication_failure_handler(Arc::new(
            AuthenticationEntryPointFailureHandler::new(authentication_entry_point),
        ));
    }

    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.base
            .set_security_context_holder_strategy(security_context_holder_strategy);
    }

    pub fn set_security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) {
        self.base
            .set_security_context_repository(security_context_repository);
    }
}

impl Deref for BearerTokenAuthenticationFilter {
    type Target = BaseAuthenticationProcessingFilter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BearerTokenAuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl BaseAuthenticationProcessingFilterExt for BearerTokenAuthenticationFilter {}

#[async_trait]
impl HttpFilter for BearerTokenAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        BaseAuthenticationProcessingFilter::do_filter(request, response, filter_chain, self).await
    }
}

impl Named for BearerTokenAuthenticationFilter {
    fn name(&self) -> &str {
        "BearerTokenAuthenticationFilter"
    }
}

impl fmt::Debug for BearerTokenAuthenticationFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BearerTokenAuthenticationFilter")
            .field(
                "authentication_manager",
                &self.authentication_manager.is_some(),
            )
            .finish()
    }
}
