use std::sync::Arc;

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
use tracing::debug;

use crate::{
    authorization::{AuthenticationTrustResolver, DefaultAuthenticationTrustResolver},
    core::context::{SecurityContextHolder, SecurityContextHolderStrategy},
    web::{
        authentication::{
            session::SessionAuthenticationStrategy, AuthenticationFailureHandler,
            SimpleUrlAuthenticationFailureHandler,
        },
        context::SecurityContextRepository,
        session::InvalidSessionStrategy,
    },
};

/// Detects that a user has been authenticated since the start of the request and, if they
/// have, calls the configured `SessionAuthenticationStrategy` to perform any
/// session-related activity such as activating session-fixation protection mechanisms or
/// checking for multiple concurrent logins.
#[derive(Clone)]
pub struct SessionManagementFilter {
    /// The security context holder strategy to use.
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    /// The security context repository to use.
    security_context_repository: Arc<dyn SecurityContextRepository>,

    /// The session authentication strategy to invoke when a user authenticates.
    session_authentication_strategy: Arc<dyn SessionAuthenticationStrategy>,

    /// The trust resolver used to determine if an authentication is authenticated.
    /// Defaults to `DefaultAuthenticationTrustResolver`.
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,

    /// Optional strategy to handle invalid (expired) session IDs.
    /// If not set, an invalid session request will simply pass through the filter chain.
    invalid_session_strategy: Option<Arc<dyn InvalidSessionStrategy>>,

    /// The handler invoked when the session authentication strategy raises an
    /// `AuthenticationError`, indicating the user is not allowed to be authenticated
    /// for this session (typically because they already have too many sessions open).
    failure_handler: Arc<dyn AuthenticationFailureHandler>,
}

impl SessionManagementFilter {
    /// Constant used to ensure a single invocation of this filter per request.
    const FILTER_APPLIED: &'static str = "__next_web_security_session_mgmt_filter_applied";

    /// Attribute key used to store the ID of the session referenced by the client
    /// (e.g., from a cookie or URL rewriting). Session managers should set this
    /// attribute so that `SessionManagementFilter` can detect invalid/expired
    /// session IDs.
    const REQUESTED_SESSION_ID_ATTR: &'static str = "__next_web_security_requested_session_id";

    /// Attribute key indicating whether the session ID referenced by the client
    /// is valid (i.e., the session exists and has not expired). Session managers
    /// should set this attribute accordingly.
    const REQUESTED_SESSION_ID_VALID_ATTR: &'static str =
        "__next_web_security_requested_session_id_valid";

    /// Creates a new `SessionManagementFilter` with the given security context repository
    /// and session authentication strategy.
    ///
    /// # Parameters
    /// * `security_context_repository` - The repository used to load and save the
    ///   security context. Must not be null.
    /// * `session_authentication_strategy` - The strategy used to perform session-related
    ///   activity (e.g., session-fixation protection, concurrency control) when a user
    ///   authenticates. Must not be null.
    pub fn new(
        security_context_repository: Arc<dyn SecurityContextRepository>,
        session_authentication_strategy: Arc<dyn SessionAuthenticationStrategy>,
    ) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            security_context_repository,
            session_authentication_strategy,
            trust_resolver: Arc::new(DefaultAuthenticationTrustResolver),
            invalid_session_strategy: None,
            failure_handler: Arc::new(SimpleUrlAuthenticationFailureHandler::new("/login")),
        }
    }

    /// Sets the strategy which will be invoked instead of allowing the filter chain to
    /// proceed, if the user agent requests an invalid session ID. If the property is not
    /// set, no action will be taken.
    ///
    /// # Parameters
    /// * `invalid_session_strategy` - The strategy to invoke. Typically a
    ///   `SimpleRedirectInvalidSessionStrategy`.
    pub fn set_invalid_session_strategy(
        &mut self,
        invalid_session_strategy: Arc<dyn InvalidSessionStrategy>,
    ) {
        self.invalid_session_strategy = Some(invalid_session_strategy);
    }

    /// The handler which will be invoked if the `SessionAuthenticationStrategy`
    /// raises an `AuthenticationError`, indicating that the user is not allowed
    /// to be authenticated for this session (typically because they already have
    /// too many sessions open).
    pub fn set_authentication_failure_handler(
        &mut self,
        failure_handler: Arc<dyn AuthenticationFailureHandler>,
    ) {
        self.failure_handler = failure_handler;
    }

    /// Sets the `AuthenticationTrustResolver` to be used. The default is
    /// `DefaultAuthenticationTrustResolver`.
    ///
    /// # Parameters
    /// * `trust_resolver` - The `AuthenticationTrustResolver` to use. Cannot be null.
    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.trust_resolver = trust_resolver;
    }

    /// Sets the `SecurityContextHolderStrategy` to use. The default action is to use
    /// the strategy stored in `SecurityContextHolder`.
    ///
    /// # Parameters
    /// * `security_context_holder_strategy` - The strategy to use. Cannot be null.
    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    /// Determines whether the client has sent a request for a session ID that is no
    /// longer valid (e.g., the session has expired or been invalidated).
    ///
    /// This mirrors the Servlet API's `HttpServletRequest.getRequestedSessionId()` and
    /// `HttpServletRequest.isRequestedSessionIdValid()` pattern by reading attributes
    /// set by the session management infrastructure.
    fn has_invalid_session_id(&self, request: &dyn HttpRequest) -> bool {
        let referenced_session_id = request
            .get_attribute(Self::REQUESTED_SESSION_ID_ATTR)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if referenced_session_id.is_none() {
            return false;
        }

        let is_valid = request
            .get_attribute(Self::REQUESTED_SESSION_ID_VALID_ATTR)
            .and_then(|v| v.as_boolean())
            .unwrap_or(false);

        !is_valid
    }
}

#[async_trait]
impl HttpFilter for SessionManagementFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // Ensure the filter is only applied once per request.
        if request.get_attribute(Self::FILTER_APPLIED).is_some() {
            return filter_chain.do_filter(request, response).await;
        }

        request.set_attribute(Self::FILTER_APPLIED, AnyValue::Boolean(true));

        if !self.security_context_repository.contains_context(request) {
            let context = self.security_context_holder_strategy.get_context();
            let authentication = context.as_ref().and_then(|s| s.get_authentication());

            if self
                .trust_resolver
                .is_authenticated(authentication.map(|auth| auth.as_ref()))
            {
                // The user has been authenticated during the current request, so
                // call the session strategy.
                if let Some(authentication) = authentication {
                    match self
                        .session_authentication_strategy
                        .on_authentication(authentication, request, response)
                        .await
                    {
                        Ok(()) => {
                            // Eagerly save the security context to make it available
                            // for any possible re-entrant requests which may occur
                            // before the current request completes. SEC-1396.
                            if let Some(context) =
                                self.security_context_holder_strategy.get_context()
                            {
                                self.security_context_repository
                                    .save_context(&context, request, response)
                                    .await;
                            }
                        }
                        Err(ex) => {
                            // The session strategy can reject the authentication.
                            debug!(
                                "SessionAuthenticationStrategy rejected the authentication object: {:?}",
                                ex
                            );
                            self.security_context_holder_strategy.clear_context();
                            self.failure_handler
                                .on_authentication_failure(request, response, &ex);
                            return Ok(());
                        }
                    }
                }
            } else {
                // No security context or authentication present. Check for a
                // session timeout.
                if self.has_invalid_session_id(request) {
                    if let Some(invalid_session_strategy) = &self.invalid_session_strategy {
                        debug!(
                            "Request requested invalid session id {:?}",
                            request
                                .get_attribute(Self::REQUESTED_SESSION_ID_ATTR)
                                .and_then(|v| v.as_str())
                        );
                        invalid_session_strategy
                            .on_invalid_session_detected(request, response)
                            .map_err(FilterError::from)?;
                        return Ok(());
                    }
                }
            }
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for SessionManagementFilter {
    fn name(&self) -> &str {
        "SessionManagementFilter"
    }
}
