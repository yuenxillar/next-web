use crate::core::session::{
    mgt::validating_session::ValidatingSession, Session, SessionError, SessionId, SessionValue,
};
use chrono::{DateTime, Utc};
use next_web_core::anys::any_map::AnyMap;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{atomic::AtomicI64, Arc};
use tracing::debug;
use uuid::Uuid;

#[derive(Clone)]
pub struct SimpleSession {
    id: SessionId,
    start_time: Arc<AtomicI64>,
    stop_time: Arc<AtomicI64>,
    last_access_time: Arc<AtomicI64>,
    timeout: Arc<AtomicI64>,
    expired: Arc<AtomicBool>,
    host: Option<String>,
    attributes: AnyMap<String, SessionValue>,
}

impl SimpleSession {
    const DEFAULT_GLOBAL_SESSION_TIMEOUT: i64 = 30 * 60 * 1000;

    const MILLIS_PER_SECOND: i64 = 1000;
    const MILLIS_PER_MINUTE: i64 = 60 * Self::MILLIS_PER_SECOND;
    const MILLIS_PER_HOUR: i64 = 60 * Self::MILLIS_PER_MINUTE;

    pub fn new(host: impl ToString) -> Self {
        let mut session = Self::default();
        session.host = Some(host.to_string());

        session
    }

    pub fn set_id(&mut self, id: SessionId) {
        self.id = id;
    }

    pub fn set_start_time(&mut self, start_time: i64) {
        self.start_time = Arc::new(AtomicI64::new(start_time));
    }

    pub fn set_stop_time(&mut self, stop_time: i64) {
        self.stop_time = Arc::new(AtomicI64::new(stop_time));
    }

    pub fn set_last_access_time(&self, last_access_time: i64) {
        self.last_access_time
            .store(last_access_time, Ordering::Relaxed);
    }

    pub fn set_expired(&self, expired: bool) {
        self.expired.store(expired, Ordering::Relaxed);
    }

    pub fn set_host(&mut self, host: impl ToString) {
        self.host = Some(host.to_string());
    }

    pub fn set_attributes(&mut self, attributes: AnyMap<String, SessionValue>) {
        self.attributes = attributes;
    }

    pub fn attributes(&self) -> &AnyMap<String, SessionValue> {
        &self.attributes
    }

    pub fn is_stopped(&self) -> bool {
        self.stop_time.load(Ordering::Relaxed) == 0
    }

    pub fn expire(&self) {
        self.stop();
        self.expired.store(true, Ordering::Relaxed);
    }

    pub fn is_timeout(&self) -> Result<bool, BoxError> {
        if self.expired.load(Ordering::Relaxed) {
            return Ok(true);
        }

        let timeout = self.timeout().unwrap_or_default();
        if timeout >= 0 {
            let last_access_time = match self.last_access_time() {
                Some(last_access_time) => last_access_time,
                None => return Err(format!("Last access time is not set").into()),
            };

            return Ok(last_access_time < (Utc::now().timestamp_millis() - timeout));
        } else {
            debug!(
                "No timeout for session with id [{}]].  Session is not considered expired.",
                self.id()
            );
        }

        Ok(false)
    }

    pub fn is_expired(&self) -> bool {
        self.expired.load(Ordering::Relaxed)
    }

    pub fn stop_time_stamp(&self) -> i64 {
        self.stop_time.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl Session for SimpleSession {
    fn id(&self) -> &SessionId {
        &self.id
    }

    fn start_timestamp(&self) -> i64 {
        self.start_time.load(Ordering::Relaxed)
    }

    fn last_access_time(&self) -> Option<i64> {
        let val = self.last_access_time.load(Ordering::Relaxed);
        match val {
            0 => None,
            _ => Some(val),
        }
    }

    fn timeout(&self) -> Result<i64, SessionError> {
        Ok(self.timeout.load(Ordering::Relaxed))
    }

    fn set_timeout(&self, timeout: i64) -> Result<(), SessionError> {
        self.timeout.store(timeout, Ordering::Relaxed);
        Ok(())
    }

    fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    fn touch(&self) -> Result<(), SessionError> {
        self.last_access_time
            .store(Utc::now().timestamp_millis(), Ordering::Relaxed);
        Ok(())
    }

    fn stop(&self) -> Result<(), SessionError> {
        if self.stop_time.load(Ordering::Relaxed) == 0 {
            self.stop_time
                .store(Utc::now().timestamp_millis(), Ordering::Relaxed)
        }
        Ok(())
    }

    async fn attribute_keys(&self) -> Result<Vec<String>, SessionError> {
        Ok(self.attributes().keys().await)
    }

    async fn get_attribute(&self, key: &str) -> Option<SessionValue> {
        self.attributes().get(key).await
    }

    async fn set_attribute(&self, key: &str, value: SessionValue) -> Result<(), SessionError> {
        let attr = self.attributes();
        if let SessionValue::Null = value {
            attr.remove(key).await;
        } else {
            attr.insert(key.to_string(), value).await;
        }

        Ok(())
    }

    async fn remove_attribute(&self, key: &str) -> Result<Option<SessionValue>, SessionError> {
        Ok(self.attributes().remove(key).await)
    }
}

impl ValidatingSession for SimpleSession {
    fn is_valid(&self) -> bool {
        !self.is_stopped() && !self.is_expired()
    }

    fn validate(&self) -> Result<(), SessionError> {
        if self.is_stopped() {
            return Err(SessionError::Stopped(format!("Session with id [{}]] has been explicitly stopped.  No further interaction under this session is allowed.", self.id).into()));
        }

        if self.is_timeout().unwrap_or_default() {
            self.expire();

            let last_access_time = self.last_access_time();
            let timeout = self.timeout()?;
            let id = self.id();

            let msg = format!("Session with id [{}] has expired. Last access time: {}.  Current time: {}.  Session timeout is set to
                {} seconds ({} minutes)", id,
                DateTime::from_timestamp_millis(last_access_time.unwrap()).map(|time| time.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or("Unknown".to_string()),
                Utc::now().format("%Y-%m-%d %H:%M"),
                timeout / Self::MILLIS_PER_SECOND,
                timeout / Self::MILLIS_PER_MINUTE );

            return Err(SessionError::Expired(Some(msg)));
        }

        Ok(())
    }
}

impl Default for SimpleSession {
    fn default() -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: SessionId::String(Uuid::new_v4().to_string()),
            timeout: Arc::new(AtomicI64::new(Self::DEFAULT_GLOBAL_SESSION_TIMEOUT)),
            start_time: Arc::new(AtomicI64::new(now)),
            stop_time: Arc::new(AtomicI64::new(0)),
            last_access_time: Arc::new(AtomicI64::new(now)),
            expired: Arc::new(AtomicBool::new(false)),
            attributes: AnyMap::new(),
            host: None,
        }
    }
}
