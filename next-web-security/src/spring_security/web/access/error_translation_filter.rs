use std::{any::Any, sync::Arc};
use tracing::{enabled, trace, Level};

use next_web_context::{support::MessageSourceAccessor, MessageSource};
use next_web_core::{
    async_trait,
    error::BoxError,
    filter::{FilterChainError, FilterError},
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    access::AccessDeniedError,
    authentication::AuthenticationTrustResolverImpl,
    authorization::AuthenticationTrustResolver,
    core::{
        context::{SecurityContextHolder, SecurityContextHolderStrategy},
        NextSecurityMessageSource, {AuthenticationError, AuthenticationErrorKind},
    },
    web::{
        access::{AccessDeniedHandler, AccessDeniedHandlerImpl},
        authentication_entry_point::AuthenticationEntryPoint,
        savedrequest::{HttpSessionRequestCache, RequestCache},
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
pub struct ErrorTranslationFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    access_denied_handler: Arc<dyn AccessDeniedHandler>,
    authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    authentication_trust_resolver: Arc<dyn AuthenticationTrustResolver>,
    request_cache: Arc<dyn RequestCache>,
    messages: MessageSourceAccessor,
}

impl ErrorTranslationFilter {
    pub fn new(
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
        request_cache: Arc<dyn RequestCache>,
    ) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            access_denied_handler: Arc::new(AccessDeniedHandlerImpl::default()),
            authentication_trust_resolver: Arc::new(AuthenticationTrustResolverImpl::default()),
            messages: NextSecurityMessageSource::get_accessor(),
            authentication_entry_point,
            request_cache,
        }
    }

    pub fn with_authentication_entry_point(
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> Self {
        Self::new(
            authentication_entry_point,
            Arc::new(HttpSessionRequestCache::default()),
        )
    }

    /// Handles an `AuthenticationError` by sending to the authentication entry
    /// point.
    fn handle_authentication_error(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        chain: &dyn HttpFilterChain,
        error: &AuthenticationError,
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
        chain: &dyn HttpFilterChain,
        error: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        let ctx = self.security_context_holder_strategy.get_context();
        let authentication = ctx.get_authentication();
        let auth = authentication.as_deref();

        if self.authentication_trust_resolver.is_anonymous(auth)
            || self.authentication_trust_resolver.is_remember_me(auth)
        {
            auth.map(|authentication| {
                    if enabled!(Level::TRACE) {
                        trace!(
                            "Sending {} to authentication entry point since access is denied, error: {}",
                            authentication.name(),
                            error
                        );
                    }
                });

            let mut ex = AuthenticationError::with_kind(
                self.messages.message_or_default(
                    "ErrorTranslationFilter.insufficientAuthentication",
                    None,
                    "Full authentication is required to access this resource",
                ),
                AuthenticationErrorKind::InsufficientAuthentication,
            );

            authentication.map(|auth| ex.set_authentication_request(auth));
            return self.send_start_authentication(request, response, chain, &ex);
        } else {
            auth.map(|authentication| {
                if enabled!(Level::TRACE) {
                    trace!(
                        "Sending {} to access denied handler since access is denied, error: {}",
                        authentication.name(),
                        error
                    );
                }
            });

            return self.access_denied_handler.handle(request, response, error);
        }

        Ok(())
    }

    /// Clears the security context, saves the request, and commences authentication via
    /// the entry point.
    fn send_start_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _chain: &dyn HttpFilterChain,
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
impl HttpFilter for ErrorTranslationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if let Err(filter_error) = filter_chain.do_filter(request, response).await {
            match filter_error {
                FilterError::Io(io_err) => return Err(FilterError::Io(io_err)),
                FilterError::Custom(msg) => return Err(FilterError::Custom(msg)),
                FilterError::Chain(chain_err) => match chain_err {
                    FilterChainError::Boxed(err) => {
                        return Err(FilterError::Chain(FilterChainError::Boxed(err)))
                    }
                    FilterChainError::AnyError(err) => {
                        if response.is_committed() {
                            return Err(FilterError::Custom(
                                    "Unable to handle the Next Security Error because the response is already committed.".to_string()
                                ));
                        }
                        let any_err = err.as_ref() as &dyn Any;
                        if let Some(auth_err) = any_err.downcast_ref::<AuthenticationError>() {
                            return self
                                .handle_authentication_error(
                                    request,
                                    response,
                                    filter_chain,
                                    auth_err,
                                )
                                .map_err(Into::into);
                        }
                        if let Some(auth_err) = any_err.downcast_ref::<AccessDeniedError>() {
                            return self
                                .handle_access_denied_error(
                                    request,
                                    response,
                                    filter_chain,
                                    auth_err,
                                )
                                .map_err(Into::into);
                        }
                    }
                },
            };
        }

        Ok(())
    }
}

impl Named for ErrorTranslationFilter {
    fn name(&self) -> &str {
        "ErrorTranslationFilter"
    }
}
