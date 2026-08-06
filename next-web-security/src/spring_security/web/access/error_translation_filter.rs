use std::{ops::DerefMut, sync::Arc};
use tracing::{enabled, trace, Level};

use next_web_context::{support::MessageSourceAccessor, MessageSource};
use next_web_core::{
    async_trait,
    error::BoxError,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    access::AccessDeniedError,
    authentication::{AuthenticationTrustResolverImpl, InsufficientAuthenticationError},
    authorization::AuthenticationTrustResolver,
    core::{
        authentication_error::AuthenticationError,
        context::{SecurityContextHolder, SecurityContextHolderStrategy},
        NextSecurityMessageSource,
    },
    web::{
        access::{AccessDeniedHandler, AccessDeniedHandlerImpl},
        authentication_entry_point::AuthenticationEntryPoint,
        savedrequest::{HttpSessionRequestCache, RequestCache},
        util::ErrorChainAnalyzer,
    },
};

/// Handles any `AccessDeniedError` and `AuthenticationError` thrown within the
/// filter chain.
///
/// This filter is necessary because it provides the bridge between Java Errors and
/// HTTP responses. It is solely concerned with maintaining the user interface. This
/// filter does not do any actual security enforcement.
///
/// If an `AuthenticationError` is detected, the filter will launch the
/// `authentication_entry_point`. This allows common handling of authentication failures
/// originating from Web or Method Security.
///
/// If an `AccessDeniedError` is detected, the filter will determine whether or not
/// the user is an anonymous user. If they are an anonymous user, the
/// `authentication_entry_point` will be launched. If they are not an anonymous user, the
/// filter will delegate to the `AccessDeniedHandler`. By default the filter will use
/// `AccessDeniedHandlerImpl`.
///
/// To use this filter, it is necessary to specify the following properties:
///
/// * `authentication_entry_point` indicates the handler that should commence the
///   authentication process if an `AuthenticationError` is detected. Note that this
///   may also switch the current protocol from http to https for an SSL login.
/// * `request_cache` determines the strategy used to save a request during the
///   authentication process in order that it may be retrieved and reused once the user
///   has authenticated. The default implementation is `HttpSessionRequestCache`.
#[derive(Clone)]
pub struct ErrorTranslationFilter<T = DefaultErrorChainAnalyzer> {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    access_denied_handler: Arc<dyn AccessDeniedHandler>,
    authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    authentication_trust_resolver: Arc<dyn AuthenticationTrustResolver>,
    error_chain_analyzer: T,
    request_cache: Arc<dyn RequestCache>,
    messages: MessageSourceAccessor,
}

impl<T> ErrorTranslationFilter<T> {
    pub fn new(
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
        request_cache: Arc<dyn RequestCache>,
    ) -> Self
    where
        T: DerefMut<Target = ErrorChainAnalyzer>,
        T: Default,
        T: Clone,
    {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            access_denied_handler: Arc::new(AccessDeniedHandlerImpl::default()),
            authentication_trust_resolver: Arc::new(AuthenticationTrustResolverImpl::default()),
            error_chain_analyzer: T::default(),
            messages: NextSecurityMessageSource::get_accessor(),
            authentication_entry_point,
            request_cache,
        }
    }

    pub fn with_authentication_entry_point(
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> Self
    where
        T: DerefMut<Target = ErrorChainAnalyzer>,
        T: Default,
        T: Clone,
    {
        Self::new(
            authentication_entry_point,
            Arc::new(HttpSessionRequestCache::default()),
        )
    }

    /// Handles an `AuthenticationException` by sending to the authentication entry
    /// point.
    fn handle_authentication_exception(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        chain: &mut dyn HttpFilterChain,
        error: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        trace!(
            "Sending to authentication entry point since authentication failed: {}",
            error
        );
        self.send_start_authentication(request, response, chain, error)
    }

    /// Handles an `AccessDeniedError`. If the user is anonymous or using
    /// remember-me, sends to the authentication entry point. Otherwise, delegates to the
    /// access denied handler.
    fn handle_access_denied_error(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        chain: &mut dyn HttpFilterChain,
        error: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        if let Some(ctx) = self.security_context_holder_strategy.get_context() {
            let _auth = ctx.get_authentication().map(AsRef::as_ref);

            if self.authentication_trust_resolver.is_anonymous(_auth)
                || self.authentication_trust_resolver.is_remember_me(_auth)
            {
                _auth.map(|authentication| {
                    if enabled!(Level::TRACE) {
                        trace!(
                            "Sending {} to authentication entry point since access is denied, error: {}",
                            authentication,
                            error
                        );
                    }
                });

                let msg = self.messages.message_or_default(
                    "ExceptionTranslationFilter.insufficientAuthentication",
                    None,
                    "Full authentication is required to access this resource",
                );
                let mut ex =
                    InsufficientAuthenticationError::new(&msg, Some(Box::new(exception.clone())));
                ex.set_authentication_request(authentication);
                return self.send_start_authentication(request, response, chain, &ex);
            } else {
                _auth.map(|authentication| {
                    if enabled!(Level::TRACE) {
                        trace!(
                            "Sending {} to access denied handler since access is denied, error: {}",
                            authentication,
                            error
                        );
                    }
                });

                return self.access_denied_handler.handle(request, response, error);
            }
        }

        Ok(())
    }

    /// Clears the security context, saves the request, and commences authentication via
    /// the entry point.
    fn send_start_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _chain: &mut dyn HttpFilterChain,
        reason: &AuthenticationError,
    ) -> Result<(), BoxError> {
        // SEC-112: Clear the SecurityContextHolder's Authentication, as the
        // existing Authentication is no longer considered valid
        let context = self.security_context_holder_strategy.create_empty_context();
        self.security_context_holder_strategy.set_context(context);
        self.request_cache.save_request(request, response);
        self.authentication_entry_point
            .commence(request, response, reason)?;
        Ok(())
    }

    /// Sets the `AccessDeniedHandler` to use.
    ///
    /// # Arguments
    ///
    /// * `access_denied_handler` - The handler for access denied events.
    pub fn set_access_denied_handler(
        &mut self,
        access_denied_handler: Arc<dyn AccessDeniedHandler>,
    ) {
        self.access_denied_handler = access_denied_handler;
    }

    /// Sets the `AuthenticationTrustResolver` to use.
    ///
    /// # Arguments
    ///
    /// * `authentication_trust_resolver` - The trust resolver for determining anonymous
    ///   and remember-me status.
    pub fn set_authentication_trust_resolver(
        &mut self,
        authentication_trust_resolver: Arc<dyn AuthenticationTrustResolver>,
    ) {
        self.authentication_trust_resolver = authentication_trust_resolver;
    }

    /// Sets the `ErrorChainAnalyzer` to use.
    pub fn set_error_chain_analyzer(&mut self, error_chain_analyzer: T) {
        self.error_chain_analyzer = error_chain_analyzer;
    }

    /// Sets the `MessageSource` for localized messages.
    ///
    /// # Arguments
    ///
    /// * `message_source` - The message source to use.
    pub fn set_message_source(&mut self, message_source: Arc<dyn MessageSource>) {
        self.messages = MessageSourceAccessor::new(message_source);
    }

    /// Sets the `SecurityContextHolderStrategy` to use. The default action is to use the
    /// `SecurityContextHolderStrategy` stored in `SecurityContextHolder`.
    ///
    /// # Arguments
    ///
    /// * `security_context_holder_strategy` - The strategy to use.
    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }
}

#[async_trait]
impl<T> HttpFilter for ErrorTranslationFilter<T>
where
    T: Send + Sync,
    T: 'static,

    T: DerefMut<Target = ErrorChainAnalyzer>,
    T: Default,
    T: Clone,
{
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        match filter_chain.do_filter(request, response).await {
            Ok(()) => return Ok(()),
            Err(filter_error) => {
                match filter_error {
                    FilterError::Io(io_err) => return Err(FilterError::Io(io_err)),
                    _ => {}
                };

                todo!()
            }
        }
    }
}

impl<T> Named for ErrorTranslationFilter<T> {
    fn name(&self) -> &str {
        "ErrorTranslationFilter"
    }
}

struct DefaultErrorChainAnalyzer {
    inner: ErrorChainAnalyzer,
}
