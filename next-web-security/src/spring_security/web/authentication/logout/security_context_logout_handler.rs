use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use std::sync::Arc;
use tracing::debug;

use crate::{
    core::{
        context::{security_context_holder::SecurityContextHolder, SecurityContextHolderStrategy},
        Authentication,
    },
    web::{
        authentication::logout::LogoutHandler,
        context::{HttpSessionSecurityContextRepository, SecurityContextRepository},
    },
};

/// Performs a logout by modifying the `SecurityContextHolder`.
///
/// Will also invalidate the `HttpSession` if `invalidate_http_session` is `true`
/// and the session is not `None`.
///
/// Will also remove the `Authentication` from the current `SecurityContext` if
/// `clear_authentication` is set to `true` (default).
#[derive(Clone)]
pub struct SecurityContextLogoutHandler {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    invalidate_http_session: bool,
    clear_authentication: bool,
    security_context_repository: Arc<dyn SecurityContextRepository>,
}

impl SecurityContextLogoutHandler {
    /// Returns whether the HTTP session should be invalidated on logout.
    pub fn is_invalidate_http_session(&self) -> bool {
        self.invalidate_http_session
    }

    /// Sets the `SecurityContextHolderStrategy` to use.
    ///
    /// The default action is to use the `SecurityContextHolderStrategy` stored in
    /// `SecurityContextHolder`.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }

    /// Causes the `HttpSession` to be invalidated when this `LogoutHandler` is invoked.
    ///
    /// Defaults to `true`.
    ///
    /// # Arguments
    ///
    /// * `invalidate_http_session` - `true` if you wish the session to be invalidated
    ///   (default) or `false` if it should not be.
    pub fn set_invalidate_http_session(&mut self, invalidate_http_session: bool) {
        self.invalidate_http_session = invalidate_http_session;
    }

    /// If `true`, removes the `Authentication` from the `SecurityContext` to
    /// prevent issues with concurrent requests.
    ///
    /// # Arguments
    ///
    /// * `clear_authentication` - `true` if you wish to clear the `Authentication`
    ///   from the `SecurityContext` (default) or `false` if the `Authentication`
    ///   should not be removed.
    pub fn set_clear_authentication(&mut self, clear_authentication: bool) {
        self.clear_authentication = clear_authentication;
    }

    /// Sets the `SecurityContextRepository` to use.
    ///
    /// Default is `HttpSessionSecurityContextRepository`.
    ///
    /// # Arguments
    ///
    /// * `security_context_repository` - the `SecurityContextRepository` to use.
    pub fn set_security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) {
        self.security_context_repository = security_context_repository;
    }
}

#[async_trait]
impl LogoutHandler for SecurityContextLogoutHandler {
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _authentication: Option<&Arc<dyn Authentication>>,
    ) {
        if self.invalidate_http_session {
            if let Some(session) = request.session_mut(false) {
                session.invalidate();
                debug!("Invalidated session {}", session.id());
            }
        }

        let context = self.security_context_holder_strategy.get_context();
        self.security_context_holder_strategy.clear_context();
        if self.clear_authentication {
            context.map(|ctx| ctx.set_authentication(None));
        }

        let empty_context = self.security_context_holder_strategy.create_empty_context();
        self.security_context_repository
            .save_context(&empty_context, request, response)
            .await;
    }
}

impl Default for SecurityContextLogoutHandler {
    fn default() -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            invalidate_http_session: true,
            clear_authentication: true,
            security_context_repository: Arc::new(HttpSessionSecurityContextRepository::default()),
        }
    }
}
