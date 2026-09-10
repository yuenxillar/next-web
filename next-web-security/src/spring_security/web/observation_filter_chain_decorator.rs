use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::{event, field, span, Instrument, Level, Span};

use crate::web::FilterChainDecorator;

/// Request attribute key under which the around-filter observation is stored for the
/// duration of a request.
pub const ATTRIBUTE: &str = "next.security.web.ObservationFilterChainDecorator.observation";

/// Observation name used when the security filter chain is bypassed.
pub const UNSECURED_OBSERVATION_NAME: &str = "next.security.http.unsecured.requests";

/// Observation name used when the security filter chain is applied.
pub const SECURED_OBSERVATION_NAME: &str = "next.security.http.secured.requests";

/// A [`FilterChainDecorator`] that records tracing spans and events for the chain and
/// each of its filters.
#[derive(Clone, Default)]
pub struct ObservationFilterChainDecorator;

impl ObservationFilterChainDecorator {
    /// Creates a new decorator.
    pub fn new() -> Self {
        Self
    }

    fn wrap_unsecured(original: Box<dyn HttpFilterChain>) -> Box<dyn HttpFilterChain> {
        FilterObservation::create(observation_span(
            UNSECURED_OBSERVATION_NAME,
            "unsecured request",
        ))
        .wrap_chain(original)
    }

    fn wrap_secured(original: Box<dyn HttpFilterChain>) -> Box<dyn HttpFilterChain> {
        FilterObservation::create(observation_span(
            SECURED_OBSERVATION_NAME,
            "secured request",
        ))
        .wrap_chain(original)
    }

    fn wrap(filters: Vec<Arc<dyn HttpFilter>>) -> Vec<ObservationFilter> {
        let size = filters.len();
        filters
            .into_iter()
            .enumerate()
            .map(|(index, filter)| ObservationFilter::new(filter, index + 1, size))
            .collect()
    }
}

impl FilterChainDecorator for ObservationFilterChainDecorator {
    fn decorate(&self, original: Box<dyn HttpFilterChain>) -> Box<dyn HttpFilterChain> {
        Self::wrap_unsecured(original)
    }

    fn decorate_with_filters(
        &self,
        original: Box<dyn HttpFilterChain>,
        filters: Vec<Arc<dyn HttpFilter>>,
    ) -> Box<dyn HttpFilterChain> {
        Box::new(ObservationVirtualFilterChain::new(
            Self::wrap_secured(original),
            Self::wrap(filters),
        ))
    }
}

/// An observation around a single chain or filter execution.
#[derive(Clone, Default)]
pub struct FilterObservation {
    span: Option<Span>,
}

impl FilterObservation {
    /// Creates an observation that records nothing.
    pub fn noop() -> Self {
        Self { span: None }
    }

    /// Creates an observation backed by the given tracing span.
    pub fn create(span: Span) -> Self {
        Self { span: Some(span) }
    }

    /// Returns `true` when this observation records nothing.
    pub fn is_noop(&self) -> bool {
        self.span.is_none()
    }

    /// Marks the observation as started.
    pub fn start(&self) {
        if let Some(span) = &self.span {
            emit_event(span, "start", "observation started");
        }
    }

    /// Records the given error on the observation.
    pub fn error(&self, error: &FilterError) {
        if let Some(span) = &self.span {
            span.record("error", field::display(error));
        }
    }

    /// Marks the observation as stopped.
    pub fn stop(&self) {
        if let Some(span) = &self.span {
            emit_event(span, "stop", "observation stopped");
        }
    }

    /// Wraps a chain so that it runs inside this observation.
    pub fn wrap_chain(&self, chain: Box<dyn HttpFilterChain>) -> Box<dyn HttpFilterChain> {
        if self.is_noop() {
            return chain;
        }
        Box::new(ObservedChain {
            inner: chain,
            span: self.span.clone(),
        })
    }
}

/// An observation that keeps track of the "before" and "after" phases of a filter chain.
#[derive(Clone)]
pub struct AroundFilterObservation {
    before: Option<Span>,
    after: Option<Span>,
}

impl AroundFilterObservation {
    /// Creates an observation that records nothing.
    pub fn noop() -> Self {
        Self {
            before: None,
            after: None,
        }
    }

    /// Creates an observation from the given before/after spans.
    pub fn create(before: Span, after: Span) -> Self {
        Self {
            before: Some(before),
            after: Some(after),
        }
    }

    /// Returns the span tracking the "before" phase, if any.
    pub fn before(&self) -> Option<&Span> {
        self.before.as_ref()
    }

    /// Returns the span tracking the "after" phase, if any.
    pub fn after(&self) -> Option<&Span> {
        self.after.as_ref()
    }

    /// Switches the active phase from "before" to "after".
    pub fn start(&self) {
        if let Some(after) = &self.after {
            emit_event(after, "start", "after phase started");
        }
    }

    /// Records the given error on the active phase.
    pub fn error(&self, error: &FilterError) {
        if let Some(after) = &self.after {
            after.record("error", field::display(error));
        }
    }

    /// Marks the active phase as stopped.
    pub fn stop(&self) {
        if let Some(before) = &self.before {
            emit_event(before, "stop", "before phase stopped");
        }
    }

    /// Wraps a chain so that it runs as the "after" phase.
    pub fn wrap_chain(&self, chain: Box<dyn HttpFilterChain>) -> Box<dyn HttpFilterChain> {
        match &self.after {
            Some(span) => Box::new(ObservedChain {
                inner: chain,
                span: Some(span.clone()),
            }),
            None => chain,
        }
    }
}

/// Returns the around-filter observation stored on the given request, or a no-op
/// observation when none is present.
pub fn observation(request: &dyn HttpRequest) -> AroundFilterObservation {
    request
        .get_attribute(ATTRIBUTE)
        .and_then(|value| value.as_object::<AroundFilterObservation>())
        .unwrap_or_else(AroundFilterObservation::noop)
}

/// A chain that runs its inner chain inside a tracing span.
#[derive(Clone)]
struct ObservedChain {
    inner: Box<dyn HttpFilterChain>,
    span: Option<Span>,
}

#[async_trait]
impl HttpFilterChain for ObservedChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        match &self.span {
            Some(span) => {
                self.inner
                    .do_filter(request, response)
                    .instrument(span.clone())
                    .await
            }
            None => self.inner.do_filter(request, response).await,
        }
    }
}

/// A chain that advances through the wrapped observations one filter at a time and
/// delegates to the original chain once every filter has been invoked.
#[derive(Clone)]
struct ObservationVirtualFilterChain {
    original_chain: Box<dyn HttpFilterChain>,
    additional_filters: Arc<Vec<ObservationFilter>>,
    position: Arc<AtomicUsize>,
}

impl ObservationVirtualFilterChain {
    fn new(
        original_chain: Box<dyn HttpFilterChain>,
        additional_filters: Vec<ObservationFilter>,
    ) -> Self {
        Self {
            original_chain,
            additional_filters: Arc::new(additional_filters),
            position: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl HttpFilterChain for ObservationVirtualFilterChain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let size = self.additional_filters.len();
        let current = self.position.fetch_add(1, Ordering::AcqRel);
        if current >= size {
            return self.original_chain.do_filter(request, response).await;
        }

        let filter = &self.additional_filters[current];
        filter.do_filter(request, response, self).await
    }
}

/// Wraps a single filter with an observation, recording the filter name and its
/// position within the chain.
#[derive(Clone)]
pub struct ObservationFilter {
    filter: Arc<dyn HttpFilter>,
    name: String,
    event_name: String,
    position: usize,
    size: usize,
}

impl ObservationFilter {
    /// Creates a new observation filter.
    ///
    /// # Arguments
    ///
    /// * `filter` - the filter to wrap
    /// * `position` - the one-based position of the filter within the chain
    /// * `size` - the total number of filters in the chain
    pub fn new(filter: Arc<dyn HttpFilter>, position: usize, size: usize) -> Self {
        let name = filter.name().to_string();
        let event_name = observation_event_name(&name).to_string();
        Self {
            filter,
            name,
            event_name,
            position,
            size,
        }
    }

    /// Returns the wrapped filter name.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    fn parent(request: &mut dyn HttpRequest) -> AroundFilterObservation {
        let parent =
            AroundFilterObservation::create(new_chain_span("before"), new_chain_span("after"));
        request.set_attribute(ATTRIBUTE, AnyValue::Object(Box::new(parent.clone())));
        parent
    }

    fn filter_span(&self) -> Span {
        span!(
            Level::INFO,
            "security_filter",
            filter_name = self.name.as_str(),
            chain_position = self.position as u64,
            chain_size = self.size as u64
        )
    }

    async fn wrap_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let parent = observation(request);
        if let Some(before) = parent.before() {
            before.record("chain_size", self.size as u64);
            before.record("chain_position", self.position as u64);
            before.record("filter_name", self.name.as_str());
            emit_event(before, &self.event_name, &format!("before {}", self.name));
        }

        let result = self.filter.do_filter(request, response, chain).await;

        parent.start();
        if let Some(after) = parent.after() {
            after.record("chain_size", self.size as u64);
            after.record("chain_position", (self.size - self.position + 1) as u64);
            after.record("filter_name", self.name.as_str());
            emit_event(after, &self.event_name, &format!("after {}", self.name));
        }

        result
    }
}

impl Named for ObservationFilter {
    fn name(&self) -> &str {
        &self.name
    }
}

#[async_trait]
impl HttpFilter for ObservationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if self.position == 1 {
            let _ = Self::parent(request);
        }

        let span = self.filter_span();
        self.wrap_filter(request, response, filter_chain)
            .instrument(span)
            .await
    }
}

/// Context carrying the chain position, chain size and reached filter name for a
/// single observation.
#[derive(Clone, Debug, Default)]
pub struct FilterChainObservationContext {
    filter_section: String,
    filter_name: Option<String>,
    chain_position: usize,
    chain_size: usize,
}

impl FilterChainObservationContext {
    fn new(filter_section: impl Into<String>) -> Self {
        Self {
            filter_section: filter_section.into(),
            ..Default::default()
        }
    }

    /// Creates the context for the "before" phase.
    pub fn before() -> Self {
        Self::new("before")
    }

    /// Creates the context for the "after" phase.
    pub fn after() -> Self {
        Self::new("after")
    }

    /// Returns the filter section.
    pub fn get_filter_section(&self) -> &str {
        &self.filter_section
    }

    /// Returns the reached filter name, if any.
    pub fn get_filter_name(&self) -> Option<&str> {
        self.filter_name.as_deref()
    }

    /// Sets the reached filter name.
    pub fn set_filter_name(&mut self, filter_name: impl Into<String>) {
        self.filter_name = Some(filter_name.into());
    }

    /// Returns the one-based chain position.
    pub fn get_chain_position(&self) -> usize {
        self.chain_position
    }

    /// Sets the one-based chain position.
    pub fn set_chain_position(&mut self, chain_position: usize) {
        self.chain_position = chain_position;
    }

    /// Returns the total chain size.
    pub fn get_chain_size(&self) -> usize {
        self.chain_size
    }

    /// Sets the total chain size.
    pub fn set_chain_size(&mut self, chain_size: usize) {
        self.chain_size = chain_size;
    }
}

/// Convention describing the observation name and low cardinality key values emitted
/// for a filter chain.
pub struct FilterChainObservationConvention;

impl FilterChainObservationConvention {
    /// Observation name of the filter chain.
    pub const CHAIN_OBSERVATION_NAME: &'static str = "next.security.filterchains";

    const CHAIN_POSITION_NAME: &'static str = "next.security.filterchain.position";

    const CHAIN_SIZE_NAME: &'static str = "next.security.filterchain.size";

    const FILTER_SECTION_NAME: &'static str = "next.security.reached.filter.section";

    const FILTER_NAME: &'static str = "next.security.reached.filter.name";

    /// Returns the observation name.
    pub fn name(&self) -> &'static str {
        Self::CHAIN_OBSERVATION_NAME
    }

    /// Returns the contextual observation name for the given context.
    pub fn contextual_name(&self, context: &FilterChainObservationContext) -> String {
        format!("security filterchain {}", context.get_filter_section())
    }

    /// Returns the low cardinality key values for the given context.
    pub fn low_cardinality_key_values(
        &self,
        context: &FilterChainObservationContext,
    ) -> Vec<(String, String)> {
        let filter_name = context.get_filter_name().unwrap_or_default();
        vec![
            (
                Self::CHAIN_SIZE_NAME.to_string(),
                context.get_chain_size().to_string(),
            ),
            (
                Self::CHAIN_POSITION_NAME.to_string(),
                context.get_chain_position().to_string(),
            ),
            (
                Self::FILTER_SECTION_NAME.to_string(),
                context.get_filter_section().to_string(),
            ),
            (Self::FILTER_NAME.to_string(), filter_name.to_string()),
        ]
    }

    /// Returns whether the given context is supported.
    pub fn supports_context(&self, _context: &FilterChainObservationContext) -> bool {
        true
    }
}

fn observation_span(name: &'static str, contextual_name: &'static str) -> Span {
    span!(
        Level::INFO,
        "security_observation",
        observation_name = name,
        contextual_name = contextual_name,
        error = field::Empty
    )
}

fn new_chain_span(section: &'static str) -> Span {
    span!(
        Level::INFO,
        "security_filterchain",
        filter_section = section,
        chain_size = field::Empty,
        chain_position = field::Empty,
        filter_name = field::Empty,
        error = field::Empty
    )
}

fn emit_event(span: &Span, event_name: &str, detail: &str) {
    event!(
        parent: span,
        Level::INFO,
        observation_event = event_name,
        detail = detail
    );
}

fn observation_event_name(name: &str) -> &str {
    match name {
        "DisableEncodeUrlFilter" => "session.urlencoding",
        "ForceEagerSessionCreationFilter" => "session.eagercreate",
        "ChannelProcessingFilter" => "access.channel",
        "WebAsyncManagerIntegrationFilter" => "context.async",
        "SecurityContextHolderFilter" => "context.holder",
        "SecurityContextPersistenceFilter" => "context.management",
        "HeaderWriterFilter" => "header",
        "CorsFilter" => "cors",
        "CsrfFilter" => "csrf",
        "LogoutFilter" => "logout",
        "OAuth2AuthorizationRequestRedirectFilter" => "oauth2.authnrequest",
        "Saml2WebSsoAuthenticationRequestFilter" => "saml2.authnrequest",
        "X509AuthenticationFilter" => "authentication.x509",
        "J2eePreAuthenticatedProcessingFilter" => "preauthentication.j2ee",
        "RequestHeaderAuthenticationFilter" => "preauthentication.header",
        "RequestAttributeAuthenticationFilter" => "preauthentication.attribute",
        "WebSpherePreAuthenticatedProcessingFilter" => "preauthentication.websphere",
        "CasAuthenticationFilter" => "cas.authentication",
        "OAuth2LoginAuthenticationFilter" => "oauth2.authentication",
        "Saml2WebSsoAuthenticationFilter" => "saml2.authentication",
        "UsernamePasswordAuthenticationFilter" => "authentication.form",
        "DefaultLoginPageGeneratingFilter" => "page.login",
        "DefaultLogoutPageGeneratingFilter" => "page.logout",
        "ConcurrentSessionFilter" => "session.concurrent",
        "DigestAuthenticationFilter" => "authentication.digest",
        "BearerTokenAuthenticationFilter" => "authentication.bearer",
        "BasicAuthenticationFilter" => "authentication.basic",
        "RequestCacheAwareFilter" => "requestcache",
        "SecurityContextHolderAwareRequestFilter" => "context.servlet",
        "JaasApiIntegrationFilter" => "jaas",
        "RememberMeAuthenticationFilter" => "authentication.rememberme",
        "AnonymousAuthenticationFilter" => "authentication.anonymous",
        "OAuth2AuthorizationCodeGrantFilter" => "oauth2.client.code",
        "SessionManagementFilter" => "session.management",
        "ExceptionTranslationFilter" => "access.exceptions",
        "FilterSecurityInterceptor" => "access.request",
        "AuthorizationFilter" => "authorization",
        "SwitchUserFilter" => "authentication.switch",
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use axum::{body::Body, extract::Request as AxumRequest, response::Response as AxumResponse};
    use next_web_core::async_trait;
    use tokio::sync::Mutex;

    use super::*;

    #[derive(Clone)]
    struct RecordingChain {
        called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl HttpFilterChain for RecordingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[derive(Clone)]
    struct RecordingFilter {
        name: String,
        order: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingFilter {
        fn new(name: &str, order: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                order,
            }
        }
    }

    impl Named for RecordingFilter {
        fn name(&self) -> &str {
            &self.name
        }
    }

    #[async_trait]
    impl HttpFilter for RecordingFilter {
        async fn do_filter(
            &self,
            request: &mut dyn HttpRequest,
            response: &mut dyn HttpResponse,
            filter_chain: &dyn HttpFilterChain,
        ) -> Result<(), FilterError> {
            self.order.lock().await.push(self.name.clone());
            filter_chain.do_filter(request, response).await
        }
    }

    fn test_request_response() -> (AxumRequest, AxumResponse) {
        let mut request = AxumRequest::builder().body(Body::empty()).unwrap();
        request.ready();
        (request, AxumResponse::new(Body::empty()))
    }

    #[tokio::test]
    async fn decorate_runs_the_original_chain() {
        let called = Arc::new(AtomicBool::new(false));
        let decorator = ObservationFilterChainDecorator::new();
        let decorated = decorator.decorate(Box::new(RecordingChain {
            called: called.clone(),
        }));

        let (mut request, mut response) = test_request_response();
        decorated
            .do_filter(&mut request, &mut response)
            .await
            .unwrap();

        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn decorate_with_filters_wraps_each_filter_in_order() {
        let called = Arc::new(AtomicBool::new(false));
        let order = Arc::new(Mutex::new(Vec::new()));
        let filters: Vec<Arc<dyn HttpFilter>> = vec![
            Arc::new(RecordingFilter::new("FirstFilter", order.clone())),
            Arc::new(RecordingFilter::new("SecondFilter", order.clone())),
        ];

        let decorator = ObservationFilterChainDecorator::new();
        let decorated = decorator.decorate_with_filters(
            Box::new(RecordingChain {
                called: called.clone(),
            }),
            filters,
        );

        let (mut request, mut response) = test_request_response();
        decorated
            .do_filter(&mut request, &mut response)
            .await
            .unwrap();

        assert_eq!(
            *order.lock().await,
            vec!["FirstFilter".to_string(), "SecondFilter".to_string()]
        );
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn observation_returns_noop_without_attribute() {
        let mut request = AxumRequest::builder().body(Body::empty()).unwrap();
        request.ready();

        let observation = observation(&request);

        assert!(observation.before().is_none());
        assert!(observation.after().is_none());
    }

    #[test]
    fn maps_known_filter_names_to_event_names() {
        assert_eq!(observation_event_name("CsrfFilter"), "csrf");
        assert_eq!(
            observation_event_name("AuthorizationFilter"),
            "authorization"
        );
        assert_eq!(observation_event_name("CustomFilter"), "CustomFilter");
    }

    #[test]
    fn convention_exposes_key_values() {
        let mut context = FilterChainObservationContext::before();
        context.set_chain_size(3);
        context.set_chain_position(1);
        context.set_filter_name("CsrfFilter");

        let convention = FilterChainObservationConvention;
        assert_eq!(
            convention.name(),
            FilterChainObservationConvention::CHAIN_OBSERVATION_NAME
        );
        assert!(convention.supports_context(&context));
        assert_eq!(
            convention.contextual_name(&context),
            "security filterchain before"
        );

        let key_values = convention.low_cardinality_key_values(&context);
        assert!(key_values
            .iter()
            .any(|(key, _)| key == "next.security.filterchain.size"));
        assert!(key_values.iter().any(|(_, value)| value == "CsrfFilter"));
    }
}
