use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::{debug, error, trace};

use crate::{
    core::{
        authz::authorization_error::AuthorizationError,
        cache::{
            cache_manager_aware::CacheManagerAware, default_cache_manager::DefaultCacheManager,
        },
        event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus},
        session::{
            mgt::{
                default_session_manager::DefaultSessionManager,
                native_session_manager::NativeSessionManagerExt, session_context::SessionContext,
                session_manager::SessionManager,
                validating_session_manager::ValidatingSessionManagerExt,
            },
            Session, SessionError, SessionId,
        },
    },
    web::{Cookie, SimpleCookie},
};

#[derive(Clone)]
pub struct DefaultWebSessionManager {
    session_id_cookie: Arc<dyn Cookie>,
    default_session_manager: DefaultSessionManager,
    session_id_cookie_enabled: bool,
    session_id_url_rewriting_enabled: bool,
}

impl DefaultWebSessionManager {
    pub const REFERENCED_SESSION_ID_SOURCE: &str = stringify!(format!(
        "{}REFERENCED_SESSION_ID_SOURCE",
        std::std::any::type_name::<Self>()
    ));

    pub const REFERENCED_SESSION_IS_NEW: &str = stringify!(format!(
        "{}_REFERENCED_SESSION_IS_NEW",
        std::std::any::type_name::<Self>()
    ));

    pub const REFERENCED_SESSION_ID_IS_VALID: &str = stringify!(format!(
        "{}_REQUESTED_SESSION_ID_VALID",
        std::std::any::type_name::<Self>()
    ));

    pub const REFERENCED_SESSION_ID: &str = stringify!(format!(
        "{}_REQUESTED_SESSION_ID",
        std::std::any::type_name::<Self>()
    ));

    pub const SESSION_ID_URL_REWRITING_ENABLED: &str = stringify!(format!(
        "{}_SESSION_ID_URL_REWRITING_ENABLED",
        std::std::any::type_name::<Self>()
    ));

    pub const DEFAULT_SESSION_ID_NAME: &str = "JSESSIONID";
    const COOKIE_SESSION_ID_SOURCE: &str = "cookie";
    const URL_SESSION_ID_SOURCE: &str = "url";
}
impl DefaultWebSessionManager {
    pub fn get_session_id_cookie(&self) -> &Arc<dyn Cookie> {
        &self.session_id_cookie
    }

    pub fn set_session_id_cookie<T: Cookie + 'static>(&mut self, cookie: T) {
        self.session_id_cookie = Arc::new(cookie);
    }

    pub fn is_session_id_cookie_enabled(&self) -> bool {
        self.session_id_cookie_enabled
    }

    pub fn set_session_id_cookie_enabled(&mut self, enabled: bool) {
        self.session_id_cookie_enabled = enabled;
    }

    pub fn is_session_id_url_rewriting_enabled(&self) -> bool {
        self.session_id_url_rewriting_enabled
    }

    pub fn set_session_id_url_rewriting_enabled(&mut self, enabled: bool) {
        self.session_id_url_rewriting_enabled = enabled;
    }

    fn store_session_id(
        &self,
        session_id: &SessionId,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), &'static str> {
        // 校验id
        if !match session_id {
            SessionId::String(s) => !s.is_empty(),
            SessionId::Number(n) => *n != 0,
            _ => true,
        } {
            return Err("session_id cannot be null when persisting for subsequent requests.");
        }

        let mut cookie = SimpleCookie::from(self.session_id_cookie.as_ref());
        cookie.set_value(session_id.to_string());
        cookie.save_to(req, resp);

        trace!(
            "Set session ID cookie for session with id {}",
            session_id.to_string()
        );

        Ok(())
    }

    fn remove_session_id_cookie(&self, req: &dyn HttpRequest, resp: &mut dyn HttpResponse) {
        self.get_session_id_cookie().remove_from(req, resp);
    }

    fn get_session_id_cookie_value(
        &self,
        req: &dyn HttpRequest,
        resp: &dyn HttpResponse,
    ) -> Option<String> {
        if !self.is_session_id_cookie_enabled() {
            debug!("Session ID cookie is disabled - session id will not be acquired from a request cookie.");
            return None;
        }

        self.get_session_id_cookie().read_value(req, resp)
    }

    fn get_uri_path_segment_param_value(
        &self,
        req: &dyn HttpRequest,
        param_name: &str,
    ) -> Option<String> {
        let uri = req.uri().path();
        if uri.is_empty() {
            return None;
        }

        // Step 1: Remove query string (everything after '?')
        let uri = match uri.find('?') {
            Some(idx) => &uri[..idx],
            None => uri,
        };

        // Step 2: Find first ';' to check for path segment params
        let semicolon_idx = uri.find(';')?;

        // Step 3: Extract substring after first ';'
        let params = &uri[semicolon_idx + 1..];

        // Step 4: Build token: "paramName="
        let token = format!("{}=", param_name);

        // Step 5: Find *last* occurrence of token (to get latest param)
        let token_idx = params.rfind(&token)?;

        // Step 6: Extract value after token
        let mut value = &params[token_idx + token.len()..];

        // Step 7: Truncate at next ';' if exists (strip subsequent params)
        if let Some(next_semi) = value.find(';') {
            value = &value[..next_semi];
        }

        // Return owned String (clone only if needed)
        Some(value.to_string())
    }

    fn get_session_id_name(&self) -> &str {
        let name = self.session_id_cookie.get_name();
        name.unwrap_or(Self::DEFAULT_SESSION_ID_NAME)
    }

    pub fn get_session_id(
        &self,
        req: &mut dyn HttpRequest,
        resp: &dyn HttpResponse,
    ) -> Option<SessionId> {
        let id = self.get_session_id_cookie_value(req, resp);

        let mut r_id = None;
        match id {
            Some(_id) => {
                req.set_attribute(
                    Self::REFERENCED_SESSION_ID_SOURCE,
                    AnyValue::String(Self::COOKIE_SESSION_ID_SOURCE.to_string()),
                );
            }
            None => {
                r_id = self.get_uri_path_segment_param_value(req, Self::DEFAULT_SESSION_ID_NAME);
                match r_id.as_ref() {
                    Some(_id) => {
                        req.set_attribute(
                            Self::REFERENCED_SESSION_ID_SOURCE,
                            AnyValue::String(Self::URL_SESSION_ID_SOURCE.to_string()),
                        );
                    }
                    None => {
                        let name = self.get_session_id_name();
                        let query_string = req.query();
                        if let Some(q_str) = query_string.as_ref() {
                            if q_str.contains(name) {
                                r_id = req.get_parameter(name).map(ToString::to_string);
                            }
                        }

                        if r_id.is_none() {
                            if let Some(q_str) = query_string.as_ref() {
                                let _name = name.to_lowercase();
                                if q_str.contains(&_name) {
                                    r_id = req
                                        .get_parameter(&_name.to_lowercase())
                                        .map(ToString::to_string);
                                }
                            }
                        }
                    }
                };

                if r_id.is_some() {
                    req.set_attribute(
                        Self::REFERENCED_SESSION_ID_SOURCE,
                        AnyValue::String(Self::URL_SESSION_ID_SOURCE.to_string()),
                    );
                }
            }
        };
        if let Some(r_id) = r_id.as_ref() {
            req.set_attribute(Self::REFERENCED_SESSION_ID, AnyValue::String(r_id.clone()));
            // automatically mark it valid here.  If it is invalid, the
            // onUnknownSession method below will be invoked and we'll remove the attribute at that time.
            req.set_attribute(
                Self::REFERENCED_SESSION_ID_IS_VALID,
                AnyValue::Boolean(true),
            );
        }
        req.set_attribute(
            Self::SESSION_ID_URL_REWRITING_ENABLED,
            AnyValue::Boolean(self.is_session_id_url_rewriting_enabled()),
        );

        r_id.map(|sid| SessionId::String(sid))
    }

    fn _on_invalidation(&self, req: &mut dyn HttpRequest, resp: &mut dyn HttpResponse) {
        req.remove_attribute(Self::REFERENCED_SESSION_ID_IS_VALID);
        self.remove_session_id_cookie(req, resp);
    }
}

#[async_trait]
impl ValidatingSessionManagerExt for DefaultWebSessionManager {
    async fn on_expiration(
        &self,
        session: &Arc<dyn Session>,
        error: SessionError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        self.default_session_manager
            .validating_session_manager
            .on_expiration(session, error, req, resp)
            .await;
        self._on_invalidation(req, resp);
    }
    async fn on_invalidation(
        &self,
        session: &dyn Session,
        ise: SessionError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        self.default_session_manager
            .validating_session_manager
            .on_invalidation(session, ise, req, resp)
            .await;
        self._on_invalidation(req, resp);
    }

    async fn get_active_sessions(&self) -> Vec<&Arc<dyn Session>> {
        self.default_session_manager.get_active_sessions().await
    }
}

#[async_trait]
impl SessionManager for DefaultWebSessionManager {
    async fn start(
        &self,
        context: &dyn SessionContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<Box<dyn Session>, AuthorizationError> {
        self.default_session_manager.start(context, req, resp).await
    }

    async fn get_session(&self, id: &SessionId) -> Result<Arc<dyn Session>, SessionError> {
        self.default_session_manager.do_get_session(id).await
    }
}

#[async_trait]
impl NativeSessionManagerExt for DefaultWebSessionManager {
    async fn on_start(
        &self,
        session: &Arc<dyn Session>,
        _ctx: &dyn SessionContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        if self.is_session_id_cookie_enabled() {
            let sesssion_id = session.id();
            if let Err(s) = self.store_session_id(sesssion_id, req, resp) {
                error!("Failed to store session ID cookie: {}", s);
            }
        } else {
            debug!(
                "Session ID cookie is disabled.  No cookie has been set for new session with id {}",
                session.id()
            );
        }

        req.remove_attribute(Self::REFERENCED_SESSION_ID_SOURCE);
        req.set_attribute(Self::REFERENCED_SESSION_IS_NEW, AnyValue::Boolean(true));
    }

    async fn on_stop(
        &self,
        session: &Arc<dyn Session>,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        self.default_session_manager.on_change(session).await;
        self.remove_session_id_cookie(req, resp);
    }

    async fn on_change(&self, session: &Arc<dyn Session>) {
        self.default_session_manager.on_change(session).await
    }
}

impl EventBusAware<DefaultEventBus> for DefaultWebSessionManager {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        self.default_session_manager.set_event_bus(event_bus);
    }
}

impl CacheManagerAware<DefaultCacheManager> for DefaultWebSessionManager {
    fn set_cache_manager(&mut self, cache_manager: DefaultCacheManager) {
        self.default_session_manager
            .set_cache_manager(cache_manager);
    }
}

impl Deref for DefaultWebSessionManager {
    type Target = DefaultSessionManager;

    fn deref(&self) -> &Self::Target {
        &self.default_session_manager
    }
}

impl DerefMut for DefaultWebSessionManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.default_session_manager
    }
}

impl Default for DefaultWebSessionManager {
    fn default() -> Self {
        let mut cookie = SimpleCookie::new(Self::DEFAULT_SESSION_ID_NAME);
        cookie.set_http_only(true);
        Self {
            session_id_cookie: Arc::new(cookie),
            default_session_manager: Default::default(),
            session_id_cookie_enabled: true,
            session_id_url_rewriting_enabled: false,
        }
    }
}
