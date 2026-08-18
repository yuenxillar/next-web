use std::{fmt::Debug, sync::Arc};

use next_web_context::{support::MessageSourceAccessor, MessageSource};
use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::{
        session::{SessionInformation, SessionRegistry},
        Authentication, NextSecurityMessageSource, {AuthenticationError, AuthenticationErrorKind},
    },
    web::authentication::session::{session_limit_of, SessionAuthenticationStrategy, SessionLimit},
};

/// Strategy which handles concurrent session-control.
///
/// When invoked following an authentication, it will check whether the user in question
/// should be allowed to proceed, by comparing the number of sessions they already have
/// active with the configured `maximum_sessions` value. The `SessionRegistry` is used as
/// the source of data on authenticated users and session data.
///
/// If a user has reached the maximum number of permitted sessions, the behaviour depends
/// on the `exception_if_max_exceeded` property. The default behaviour is to expire any
/// sessions that exceed the maximum number of permitted sessions, starting with the least
/// recently used sessions. The expired sessions will be invalidated by the
/// `ConcurrentSessionFilter` if accessed again. If `exception_if_max_exceeded` is set to
/// `true`, however, the user will be prevented from starting a new authenticated session.
///
/// This strategy can be injected into both the `SessionManagementFilter` and instances of
/// `AbstractAuthenticationProcessingFilter` (typically
/// `UsernamePasswordAuthenticationFilter`), but is typically combined with
/// `RegisterSessionAuthenticationStrategy` using `CompositeSessionAuthenticationStrategy`.
pub struct ConcurrentSessionControlAuthenticationStrategy {
    messages: MessageSourceAccessor,
    session_registry: Arc<dyn SessionRegistry>,
    error_if_maximum_exceeded: bool,
    session_limit: SessionLimit,
}

impl ConcurrentSessionControlAuthenticationStrategy {
    /// Creates a new instance.
    ///
    /// # Arguments
    ///
    /// * `session_registry` - The session registry which should be updated when the
    ///   authenticated session is changed.
    pub fn new(session_registry: Arc<dyn SessionRegistry>) -> Self {
        Self {
            messages: NextSecurityMessageSource::get_accessor(),
            session_registry,
            error_if_maximum_exceeded: false,
            session_limit: session_limit_of(1),
        }
    }

    /// Method intended for use by subclasses to override the maximum number of sessions
    /// that are permitted for a particular authentication. The default implementation
    /// simply returns the `maximum_sessions` value for the bean.
    ///
    /// # Arguments
    ///
    /// * `authentication` - The authentication to determine the maximum sessions for.
    ///
    /// # Returns
    ///
    /// Either -1 meaning unlimited, or a positive integer to limit (never zero).
    fn get_maximum_sessions_for_this_user(&self, authentication: &dyn Authentication) -> i32 {
        (self.session_limit.as_ref())(authentication)
    }

    /// Allows subclasses to customise behaviour when too many sessions are detected.
    ///
    /// # Arguments
    ///
    /// * `sessions` - All unexpired sessions associated with the principal.
    /// * `allowable_sessions` - The number of concurrent sessions the user is allowed to
    ///   have.
    /// * `registry` - An instance of the `SessionRegistry` for subclass use.
    fn allowable_sessions_exceeded(
        &self,
        mut sessions: Vec<SessionInformation>,
        allowable_sessions: i32,
        _registry: &Arc<dyn SessionRegistry>,
    ) -> Result<(), AuthenticationError> {
        if self.error_if_maximum_exceeded {
            return Err(AuthenticationError::with_kind(
                self.messages.message_or_default(
                    "ConcurrentSessionControlAuthenticationStrategy.exceededAllowed",
                    Some(&[&allowable_sessions]),
                    "Maximum sessions of {0} for this principal exceeded",
                ),
                AuthenticationErrorKind::SessionAuthentication,
            ));
        }

        // Determine least recently used sessions, and mark them for invalidation.
        sessions.sort_by(|a, b| a.last_request().cmp(&b.last_request()));

        let maximum_sessions_exceeded_by =
            (sessions.len() as i32 - allowable_sessions + 1).max(0) as usize;
        let len = sessions.len();
        let sessions_to_be_expired = &mut sessions[..maximum_sessions_exceeded_by.min(len)];

        for session in sessions_to_be_expired {
            session.expire_now();
        }

        Ok(())
    }

    /// Sets the `error_if_maximum_exceeded` property, which determines whether the
    /// user should be prevented from opening more sessions than allowed. If set to
    /// `true`, a `SessionAuthenticationException` will be raised which means the user
    /// authenticating will be prevented from authenticating. If set to `false`, the user
    /// that has already authenticated will be forcibly logged out.
    ///
    /// # Arguments
    ///
    /// * `error_if_maximum_exceeded` - Defaults to `false`.
    pub fn set_error_if_maximum_exceeded(&mut self, error_if_maximum_exceeded: bool) {
        self.error_if_maximum_exceeded = error_if_maximum_exceeded;
    }

    /// Sets the `maximum_sessions` property. The default value is 1. Use -1 for
    /// unlimited sessions.
    ///
    /// # Arguments
    ///
    /// * `maximum_sessions` - The maximum number of permitted sessions a user can have
    ///   open simultaneously.
    pub fn set_maximum_sessions(&mut self, maximum_sessions: i32) {
        self.session_limit = session_limit_of(maximum_sessions);
    }

    /// Sets the `session_limit` property. The default value is 1. Use -1 for unlimited
    /// sessions.
    ///
    /// # Arguments
    ///
    /// * `session_limit` - The session limit strategy.
    pub fn set_maximum_sessions_with_limit(&mut self, session_limit: SessionLimit) {
        self.session_limit = session_limit;
    }

    /// Sets the `MessageSource` used for reporting errors back to the user when the user
    /// has exceeded the maximum number of authentications.
    ///
    /// # Arguments
    ///
    /// * `message_source` - The message source to use.
    pub fn set_message_source(&mut self, message_source: Arc<dyn MessageSource>) {
        self.messages = MessageSourceAccessor::new(message_source);
    }
}

#[async_trait]
impl SessionAuthenticationStrategy for ConcurrentSessionControlAuthenticationStrategy {
    async fn on_authentication(
        &self,
        authentication: &Arc<dyn Authentication>,
        req: &mut dyn HttpRequest,
        _resp: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError> {
        let allowed_sessions = self.get_maximum_sessions_for_this_user(authentication.as_ref());
        if allowed_sessions == -1 {
            // We permit unlimited logins
            return Ok(());
        }

        let principal = match authentication.principal() {
            Some(principal) => principal,
            None => {
                return Err(AuthenticationError::new(
                    "Authentication.principal() cannot be none".to_string(),
                ))
            }
        };

        let sessions = self
            .session_registry
            .all_sessions(principal, false)
            .await;
        let session_count = sessions.len() as i32;

        if session_count < allowed_sessions {
            // They haven't got too many login sessions running at present
            return Ok(());
        }

        if session_count == allowed_sessions {
            if let Some(session) = req.session() {
                // Only permit it though if this request is associated with one of the
                // already registered sessions
                let session_id = session.id();
                for si in &sessions {
                    if si.session_id() == session_id {
                        return Ok(());
                    }
                }
            }
            // If the session is null, a new one will be created by the parent class,
            // exceeding the allowed number
        }

        self.allowable_sessions_exceeded(sessions, allowed_sessions, &self.session_registry)
    }
}

impl Debug for ConcurrentSessionControlAuthenticationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConcurrentSessionControlAuthenticationStrategy")
            .field("messages", &"none")
            .field("session_registry", &"none")
            .field("error_if_maximum_exceeded", &self.error_if_maximum_exceeded)
            .field("session_limit", &"none")
            .finish()
    }
}
