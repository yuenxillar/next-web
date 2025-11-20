use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::{
    authz::authorization_error::AuthorizationError,
    cache::{
        cache_manager::CacheManager, cache_manager_aware::CacheManagerAware,
        default_cache_manager::DefaultCacheManager,
    },
    event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus},
    session::{
        mgt::{
            default_validating_session_manager::DefaultValidatingSessionManager,
            eis::{memory_session_dao::MemorySessionDAO, session_dao::SessionDAO},
            native_session_manager::NativeSessionManagerExt,
            session_context::SessionContext,
            session_factory::SessionFactory,
            session_manager::SessionManager,
            simple_session::SimpleSession,
            simple_session_factory::SimpleSessionFactory,
            validating_session_manager::ValidatingSessionManagerExt,
        },
        Session, SessionError, SessionId, SessionValue,
    },
};

use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tracing::trace;

#[derive(Clone)]
pub struct DefaultSessionManager {
    session_dao: Arc<dyn SessionDAO>,
    session_factory: Arc<dyn SessionFactory>,
    cache_manager: Option<Arc<dyn CacheManager>>,
    delete_invalid_sessions: bool,

    pub(crate) validating_session_manager: DefaultValidatingSessionManager,
}

impl DefaultSessionManager {
    pub fn set_session_dao<T>(&mut self, session_dao: T)
    where
        T: SessionDAO + 'static,
    {
        self.session_dao = Arc::new(session_dao);
        self.apply_cache_manager_to_session_dao();
    }

    pub fn get_session_dao(&self) -> &Arc<dyn SessionDAO> {
        &self.session_dao
    }

    pub fn get_session_factory(&self) -> &Arc<dyn SessionFactory> {
        &self.session_factory
    }

    pub fn set_session_factory<T>(&mut self, session_factory: T)
    where
        T: SessionFactory + 'static,
    {
        self.session_factory = Arc::new(session_factory);
    }

    pub fn is_delete_invalid_sessions(&self) -> bool {
        self.delete_invalid_sessions
    }

    pub fn set_delete_invalid_sessions(&mut self, delete_invalid_sessions: bool) {
        self.delete_invalid_sessions = delete_invalid_sessions;
    }

    pub async fn create_session(&self, ctx: &dyn SessionContext) -> Arc<dyn Session> {
        self.validating_session_manager
            .enable_session_validation_if_necessary();

        let session = self.new_session_instance(ctx);
        self.create(session.clone()).await;

        session
    }

    pub fn new_session_instance(&self, ctx: &dyn SessionContext) -> Arc<dyn Session> {
        self.session_factory.create_session(ctx)
    }

    pub async fn create(&self, session: Arc<dyn Session>) {
        self.session_dao.create(session).await;
    }

    pub async fn do_get_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Arc<dyn Session>, SessionError> {
        self.validating_session_manager
            .enable_session_validation_if_necessary();
        trace!("Attempting to retrieve session with id {}", session_id);

        let session = self.retrieve_session_from_data_source(session_id).await;

        session
            .map(|s| {
                // self.validate(&s, session_id);
                Ok(s.clone())
            })
            .unwrap_or(Err(SessionError::NotFound))
    }

    async fn _get_session(&self, session_id: &SessionId) -> Option<&Arc<dyn Session>> {
        self.validating_session_manager
            .enable_session_validation_if_necessary();

        self.retrieve_session_from_data_source(session_id).await
    }
    pub async fn after_expired(&self, session: &dyn Session) {
        if self.is_delete_invalid_sessions() {
            self.session_dao.delete(session).await;
        }
    }

    pub async fn retrieve_session_from_data_source(
        &self,
        session_id: &SessionId,
    ) -> Option<&Arc<dyn Session>> {
        self.session_dao.read(session_id).await
    }

    fn apply_cache_manager_to_session_dao(&mut self) {
        // todo!()
    }
}

impl DefaultSessionManager {
    pub async fn start_time_stamp(&self, session_id: &SessionId) -> i64 {
        self._get_session(session_id)
            .await
            .map(|s| s.start_timestamp())
            .unwrap_or_default()
    }

    pub async fn last_access_time(&self, session_id: &SessionId) -> Option<i64> {
        self._get_session(session_id)
            .await
            .map(|s| s.last_access_time())?
    }

    pub async fn is_valid(&self, session_id: &SessionId) -> bool {
        match self.check_valid(session_id).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn check_valid(&self, session_id: &SessionId) -> Result<(), SessionError> {
        match self._get_session(session_id).await {
            Some(_) => Ok(()),
            None => Err(SessionError::NotFound),
        }
    }

    pub async fn timeout(&self, session_id: &SessionId) -> Result<i64, SessionError> {
        self._get_session(session_id)
            .await
            .map(|s| s.timeout())
            .unwrap_or(Ok(0))
    }

    pub async fn set_timeout(
        &mut self,
        session_id: &SessionId,
        max_idle_time_in_millis: i64,
    ) -> Result<(), SessionError> {
        if let Some(s) = self._get_session(session_id).await {
            s.set_timeout(max_idle_time_in_millis);
            self.on_change(s).await;
            return Ok(());
        }

        Err(SessionError::Invalid(None))
    }

    pub async fn touch(&self, session_id: &SessionId) -> Result<(), SessionError> {
        if let Some(s) = self._get_session(session_id).await {
            s.touch();
            self.on_change(s).await;
            return Ok(());
        }

        Err(SessionError::Invalid(None))
    }

    pub async fn host(&self, session_id: &SessionId) -> Option<&str> {
        self._get_session(session_id)
            .await
            .map(|s| s.host())
            .unwrap_or_default()
    }

    pub async fn stop(
        &self,
        session_id: &SessionId,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), SessionError> {
        if let Some(s) = self._get_session(session_id).await {
            s.stop();
            self.on_stop(s, req, resp).await;
            self.notify_stop(s.clone());
            self.after_stopped(s.as_ref()).await;
            return Ok(());
        }
        Err(SessionError::Invalid(None))
    }

    pub async fn attribute_keys(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<String>, SessionError> {
        if let Some(s) = self._get_session(session_id).await {
            if let Ok(value) = s.attribute_keys().await {
                if !value.is_empty() {
                    return Ok(value);
                }
                return Ok(Vec::with_capacity(0));
            }
        }

        Err(SessionError::NotFound)
    }

    pub async fn attribute(
        &self,
        session_id: &SessionId,
        key: &str,
    ) -> Result<SessionValue, SessionError> {
        if let Some(s) = self._get_session(session_id).await {
            if let Some(value) = s.get_attribute(key).await {
                return Ok(value.clone());
            }
        }

        Err(SessionError::NotFound)
    }

    pub async fn set_attribute(
        &self,
        session_id: &SessionId,
        key: &str,
        value: Option<SessionValue>,
    ) -> Result<(), SessionError> {
        match value {
            Some(session_value) => {
                if let Some(s) = self._get_session(session_id).await {
                    s.set_attribute(key, session_value).await?;

                    self.on_change(s).await;
                }
            }
            None => {
                let _ = self.remove_attribute(session_id, key).await?;
            }
        };

        Ok(())
    }

    pub async fn remove_attribute(
        &self,
        session_id: &SessionId,
        key: &str,
    ) -> Result<SessionValue, SessionError> {
        if let Some(s) = self._get_session(session_id).await {
            let removed = s.remove_attribute(key).await?;

            if let Some(value) = removed {
                self.on_change(s).await;
                return Ok(value);
            }
        }

        Err(SessionError::NotFound)
    }

    
}

#[async_trait]
impl SessionManager for DefaultSessionManager {
    async fn start(
        &self,
        context: &dyn SessionContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<Box<dyn Session>, AuthorizationError> {
        let session = self.create_session(context).await;
        self.apply_global_session_timeout(session.as_ref());
        self.on_change(&session).await;
        self.on_start(&session, context, req, resp).await;
        self.notify_start(session.as_ref());

        Ok(Box::new(self.create_exposed_session(session.as_ref())))
    }

    async fn get_session(&self, id: &SessionId) -> Result<Arc<dyn Session>, SessionError> {
        self.do_get_session(id)
            .await
            .map(|s| Arc::new(self.create_exposed_session(s.as_ref())) as Arc<dyn Session>)
    }
}

#[async_trait]
impl NativeSessionManagerExt for DefaultSessionManager {
    #[allow(unused_variables)]
    async fn on_stop(
        &self,
        session: &Arc<dyn Session>,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        if let Some(simple_session) = (session as &dyn Any).downcast_ref::<SimpleSession>() {
            let stop_tt = simple_session.stop_time_stamp();
            simple_session.set_last_access_time(stop_tt);
        }

        self.on_change(session).await;
    }

    async fn after_stopped(&self, session: &dyn Session) {
        if self.is_delete_invalid_sessions() {
            self.session_dao.delete(session).await;
        }
    }

    async fn on_change(&self, session: &Arc<dyn Session>) {
        self.session_dao.update(session.clone()).await;
    }
}

#[async_trait]
impl ValidatingSessionManagerExt for DefaultSessionManager {
    async fn on_expiration(
        &self,
        session: &Arc<dyn Session>,
        _error: SessionError,
        _req: &mut dyn HttpRequest,
        _resp: &mut dyn HttpResponse,
    ) {
        if let Some(simple_session) = (session as &dyn Any).downcast_ref::<SimpleSession>() {
            simple_session.set_expired(true);
        }

        self.on_change(session).await;
    }


    async fn get_active_sessions(&self) -> Vec<&Arc<dyn Session>> {
        self.session_dao.get_active_sessions().await
    }
}

impl CacheManagerAware<DefaultCacheManager> for DefaultSessionManager {
    fn set_cache_manager(&mut self, cache_manager: DefaultCacheManager) {
        self.cache_manager = Some(Arc::new(cache_manager));
        self.apply_cache_manager_to_session_dao();
    }
}

impl EventBusAware<DefaultEventBus> for DefaultSessionManager {
    fn set_event_bus(&mut self, event_bus: DefaultEventBus) {
        self.validating_session_manager
            .native_session_manager
            .set_event_bus(event_bus);
    }
}

impl Deref for DefaultSessionManager {
    type Target = DefaultValidatingSessionManager;

    fn deref(&self) -> &Self::Target {
        &self.validating_session_manager
    }
}

impl DerefMut for DefaultSessionManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.validating_session_manager
    }
}

impl Default for DefaultSessionManager {
    fn default() -> Self {
        Self {
            session_dao: Arc::new(MemorySessionDAO::default()),
            session_factory: Arc::new(SimpleSessionFactory::default()),
            cache_manager: None,
            delete_invalid_sessions: true,

            validating_session_manager: Default::default(),
        }
    }
}
