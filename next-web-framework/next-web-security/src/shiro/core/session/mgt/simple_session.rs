use crate::core::session::{Session, SessionError, SessionId, SessionValue};
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct SimpleSession {
    id: SessionId,
    start_time: DateTime<Utc>,
    stop_time: Option<DateTime<Utc>>,
    last_access_time: DateTime<Utc>,
    timeout: u64,
    expired: bool,
    host: Option<String>,
    attributes: Option<HashMap<String, SessionValue>>,
}

impl SimpleSession {
    const DEFAULT_GLOBAL_SESSION_TIMEOUT: u64 = 30 * 60 * 1000;

    pub fn new(host: impl ToString) -> Self {
        let mut session = Self::default();
        session.host = Some(host.to_string());

        session
    }

    pub fn set_id(&mut self, id: SessionId) {
        self.id = id;
    }

    pub fn set_start_time(&mut self, start_time: DateTime<Utc>) {
        self.start_time = start_time;
    }

    pub fn set_stop_time(&mut self, stop_time: DateTime<Utc>) {
        self.stop_time = Some(stop_time);
    }

    pub fn set_last_access_time(&mut self, last_access_time: DateTime<Utc>) {
        self.last_access_time = last_access_time;
    }

    pub fn set_expired(&mut self, expired: bool) {
        self.expired = expired;
    }

    pub fn set_host(&mut self, host: impl ToString) {
        self.host = Some(host.to_string());
    }

    pub fn set_attributes(&mut self, attributes: HashMap<String, SessionValue>) {
        self.attributes = Some(attributes);
    }
}

impl Session for SimpleSession {
    fn id(&self) -> &SessionId {
        &self.id
    }

    fn start_timestamp(&self) -> &DateTime<Utc> {
        &self.start_time
    }

    fn last_access_time(&self) -> &DateTime<Utc> {
        &self.last_access_time
    }

    fn timeout(&self) -> Result<u64, SessionError> {
        Ok(self.timeout)
    }

    fn set_timeout(&mut self, timeout: u64) -> Result<(), SessionError> {
        self.timeout = timeout;
        Ok(())
    }

    fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    fn touch(&self) -> Result<(), SessionError> {
        self.last_access_time = Utc::now();
        Ok(())
    }

    /// 显式停止（销毁）会话
    fn stop(&self) -> Result<(), SessionError> {}

    /// 获取所有属性的 key 集合
    fn attribute_keys(&self) -> Result<HashSet<String>, SessionError> {
        todo!()
    }

    /// 获取指定 key 的属性值
    fn get_attribute(&self, key: &str) -> Option<SessionValue> {
        todo!()
    }

    /// 绑定属性（key-value）
    /// 若 value 为 None，等价于 remove_attribute
    fn set_attribute(&mut self, key: &str, value: SessionValue) -> Result<(), SessionError> {
        todo!()
    }

    /// 移除指定 key 的属性
    fn remove_attribute(&mut self, key: &str) -> Result<Option<SessionValue>, SessionError> {
        todo!()
    }
}

impl Default for SimpleSession {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::String(Uuid::new_v4().to_string()),
            timeout: Self::DEFAULT_GLOBAL_SESSION_TIMEOUT,
            start_time: now.clone(),
            stop_time: None,
            last_access_time: now,
            expired: false,
            host: None,
            attributes: None,
        }
    }
}
