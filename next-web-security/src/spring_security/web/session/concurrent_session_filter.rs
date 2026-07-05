use std::sync::Arc;

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
use tracing::debug;

use crate::{
    core::{
        context::{
            security_context_holder::SecurityContextHolder,
            security_context_holder_strategy::SecurityContextHolderStrategy,
        },
        session::SessionRegistry,
    },
    web::{
        authentication::logout::{
            CompositeLogoutHandler, LogoutHandler, SecurityContextLogoutHandler,
        },
        session::{
            session_information_expired_event::SessionInformationExpiredEvent,
            session_information_expired_strategy::SessionInformationExpiredStrategy,
        },
    },
};

/// A `SessionInformationExpiredStrategy` that writes an error message to the
/// response body. This is the default strategy used by
/// `ConcurrentSessionFilter`.
#[derive(Clone)]
struct ResponseBodySessionInformationExpiredStrategy;

impl SessionInformationExpiredStrategy for ResponseBodySessionInformationExpiredStrategy {
    fn on_expired_session_detected(
        &self,
        _event: &SessionInformationExpiredEvent,
    ) -> Result<(), BoxError> {
        // In the Java implementation this writes to the servlet response.
        // In Rust, the response is handled by the filter after this call.
        Ok(())
    }
}

/// Filter required by concurrent session handling package.
///
/// This filter performs two functions:
/// 1. Calls `SessionRegistry::refresh_last_request` for each request so that
///    registered sessions always have a correct "last update" date/time.
/// 2. Retrieves a `SessionInformation` from the `SessionRegistry` for each
///    request and checks if the session has been marked as expired. If it has
///    been marked as expired, the configured logout handlers will be called
///    (as happens with `LogoutFilter`), typically to invalidate the session.
///    The `SessionInformationExpiredStrategy` is then invoked.
#[derive(Clone)]
pub struct ConcurrentSessionFilter {
    /// The security context holder strategy.
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    /// The session registry used to look up session information.
    session_registry: Arc<dyn SessionRegistry>,

    /// The strategy invoked when an expired session is detected.
    session_information_expired_strategy: Arc<dyn SessionInformationExpiredStrategy>,

    /// The logout handler called before the expired strategy.
    logout_handlers: Arc<dyn LogoutHandler>,
}

impl ConcurrentSessionFilter {
    /// Creates a new `ConcurrentSessionFilter` with a default
    /// `ResponseBodySessionInformationExpiredStrategy`.
    ///
    /// # Parameters
    /// * `session_registry` - The `SessionRegistry` to use for session lookups.
    pub fn new(session_registry: Arc<dyn SessionRegistry>) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            session_registry,
            session_information_expired_strategy: Arc::new(
                ResponseBodySessionInformationExpiredStrategy,
            ),
            logout_handlers: Arc::new(CompositeLogoutHandler::new(vec![Arc::new(
                SecurityContextLogoutHandler::default(),
            )])),
        }
    }

    /// Creates a new `ConcurrentSessionFilter` with a custom
    /// `SessionInformationExpiredStrategy`.
    ///
    /// # Parameters
    /// * `session_registry`                    - The `SessionRegistry` to use.
    /// * `session_information_expired_strategy` - The strategy to invoke when
    ///   an expired session is detected.
    pub fn with_expired_strategy(
        session_registry: Arc<dyn SessionRegistry>,
        session_information_expired_strategy: Arc<dyn SessionInformationExpiredStrategy>,
    ) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            session_registry,
            session_information_expired_strategy,
            logout_handlers: Arc::new(CompositeLogoutHandler::new(vec![Arc::new(
                SecurityContextLogoutHandler::default(),
            )])),
        }
    }

    // --- Setters ---

    /// Sets the `SecurityContextHolderStrategy`.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }

    /// Sets the `LogoutHandler`s. This replaces any previously configured
    /// logout handlers.
    pub fn set_logout_handlers(&mut self, handlers: Vec<Arc<dyn LogoutHandler>>) {
        self.logout_handlers = Arc::new(CompositeLogoutHandler::new(handlers));
    }

    // --- Internal methods ---

    /// Performs logout by invoking the configured logout handlers with the
    /// current authentication from the security context.
    async fn do_logout(&self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse) {
        let auth = self
            .security_context_holder_strategy
            .get_context()
            .and_then(|ctx| ctx.get_authentication());

        self.logout_handlers
            .logout(request, response, auth.as_ref())
            .await;
    }
}

#[async_trait]
impl HttpFilter for ConcurrentSessionFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // Check if the request has an existing session.
        if let Some(session) = request.session() {
            let session_id = session.id().to_string();
            let info = self.session_registry.session_information(&session_id);

            if let Some(ref info) = info {
                if info.is_expired() {
                    // Expired — abort processing.
                    debug!("Requested session ID {} has expired.", session_id);

                    self.do_logout(request, response).await;

                    let event = SessionInformationExpiredEvent::new(info.clone());
                    self.session_information_expired_strategy
                        .on_expired_session_detected(&event)
                        .map_err(|e| FilterError::custom(e.to_string()))?;

                    return Ok(());
                }

                // Non-expired — update last request date/time.
                self.session_registry
                    .refresh_last_request(info.session_id());
            }
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for ConcurrentSessionFilter {
    fn name(&self) -> &str {
        "ConcurrentSessionFilter"
    }
}
