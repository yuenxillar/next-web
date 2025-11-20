pub mod mgt;
pub mod proxied_session;
pub mod session_listener;

use next_web_core::async_trait;
use next_web_core::traits::any_clone::AnyClone;
use std::any::Any;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;

use crate::core::util::object::AnyObject;

/// 表示会话已失效或非法操作
#[derive(Debug, PartialEq, Eq)]
pub enum SessionError {
    Invalid(Option<String>),
    Expired(Option<String>),
    Stopped(String),
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionId {
    /// 字符串 ID
    String(String),

    /// 数字 ID
    Number(u64),
}

#[derive(Debug, Clone)]
pub enum SessionValue {
    String(String),
    Int(i64),
    Boolean(bool),
    Bytes(Vec<u8>),
    Object(Box<dyn AnyClone>),
    Null,
}


impl SessionValue {
    pub fn as_object<T: AnyClone>(&self) -> Option<&T> {
        if let SessionValue::Object(ref obj) = self {
            (obj as &dyn Any).downcast_ref::<T>()
        } else {
            None
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        if let SessionValue::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}


#[async_trait]
pub trait Session
where
    Self: Send + Sync,
    Self: Any,
{
    /// 推荐使用 String、Uuid 或 u64
    fn id(&self) -> &SessionId;

    /// 会话创建时间
    fn start_timestamp(&self) -> i64;

    /// 上次访问时间（不因调用此方法而更新）
    fn last_access_time(&self) -> Option<i64>;

    /// 获取超时时间（毫秒）。负值表示永不过期。
    /// 返回 `Err(SessionError)` 表示会话已失效。
    fn timeout(&self) -> Result<i64, SessionError>;

    /// 设置超时时间（毫秒）。负值表示永不过期。
    fn set_timeout(&self, max_idle_time_in_millis: i64) -> Result<(), SessionError>;

    /// 客户端主机（IP 或 hostname），可能为 None
    fn host(&self) -> Option<&str>;

    /// 显式更新 last_access_time 为当前时间
    fn touch(&self) -> Result<(), SessionError>;

    /// 显式停止（销毁）会话
    fn stop(&self) -> Result<(), SessionError>;

    /// 获取所有属性的 key 集合
    async fn attribute_keys(&self) -> Result<Vec<String>, SessionError>;

    /// 获取指定 key 的属性值
    async fn get_attribute(&self, key: &str) -> Option<SessionValue>;

    /// 绑定属性（key-value）
    /// 若 value 为 None，等价于 remove_attribute
    async fn set_attribute(&self, key: &str, value: SessionValue) -> Result<(), SessionError>;

    /// 移除指定 key 的属性
    async fn remove_attribute(&self, key: &str) -> Result<Option<SessionValue>, SessionError>;
}


impl Error for SessionError {}
impl Display for SessionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::Expired(msg)  => write!(f, "Session expired: {:?}", msg),
            SessionError::Invalid(msg) => write!(f, "Session invalid: {:?}", msg),
            SessionError::NotFound => write!(f, "Session not found"),
            SessionError::Stopped(msg) => write!(f, "Session stopped: {}", msg),
        }
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionId::String(id) => write!(f, "{}", id),
            Self::Number(id) => write!(f, "{}", id),
        }
    }
}
