pub mod mgt;
pub mod proxied_session;
use std::collections::HashSet;
use std::time::SystemTime;

use next_web_core::traits::any_clone::AnyClone;

/// 表示会话已失效或非法操作
#[derive(Debug)]
pub struct InvalidSessionError;

#[derive(Debug, Clone)]
pub enum SessionId {
    /// 字符串 ID
    String(String),

    /// 数字 ID
    Number(u64),

    /// 其它 ID
    Other(Box<dyn AnyClone>),
}

#[derive(Debug, Clone)]
pub enum SessionValue {
    String(String),
    Int(i64),
    Boolean(bool),
    Bytes(Vec<u8>),
    Object(Box<dyn AnyClone>),
}

/// Shiro 风格的 Session trait（Rust 版）
pub trait Session
where
    Self: Send + Sync,
{
    /// 唯一会话 ID（对应 Java 的 Serializable）
    /// 推荐使用 String、Uuid 或 u64
    fn id(&self) -> SessionId;

    /// 会话创建时间
    fn start_timestamp(&self) -> SystemTime;

    /// 上次访问时间（不因调用此方法而更新）
    fn last_access_time(&self) -> SystemTime;

    /// 获取超时时间（毫秒）。负值表示永不过期。
    /// 返回 `Err(InvalidSessionError)` 表示会话已失效。
    fn timeout(&self) -> Result<u64, InvalidSessionError>;

    /// 设置超时时间（毫秒）。负值表示永不过期。
    fn set_timeout(&mut self, max_idle_time_in_millis: u64) -> Result<(), InvalidSessionError>;

    /// 客户端主机（IP 或 hostname），可能为 None
    fn host(&self) -> Option<&str>;

    /// 显式更新 last_access_time 为当前时间
    fn touch(&mut self) -> Result<(), InvalidSessionError>;

    /// 显式停止（销毁）会话
    fn stop(&mut self) -> Result<(), InvalidSessionError>;

    /// 获取所有属性的 key 集合
    fn attribute_keys(&self) -> Result<HashSet<String>, InvalidSessionError>;

    /// 获取指定 key 的属性值
    fn get_attribute(&self, key: &str) -> Result<Option<SessionValue>, InvalidSessionError>;

    /// 绑定属性（key-value）
    /// 若 value 为 None，等价于 remove_attribute
    fn set_attribute(
        &mut self,
        key: &str,
        value: Option<SessionValue>,
    ) -> Result<(), InvalidSessionError>;

    /// 移除指定 key 的属性
    fn remove_attribute(&mut self, key: &str) -> Result<Option<SessionValue>, InvalidSessionError>;
}
