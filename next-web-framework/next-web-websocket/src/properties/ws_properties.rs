use rudi_dev::{Properties, Singleton};

/// WebSocket配置属性，用于配置WebSocket连接的相关参数
/// 
/// WebSocket configuration properties, used to configure parameters related to WebSocket connections
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.ws")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct WebSocketProperties {
    /// 最大消息大小
    /// 
    /// Maximum message size
    max_msg_size: Option<usize>,
    /// 最大写入 buff 容量大小
    /// 
    /// Maximum buffer capacity for writing
    max_write_buffer_size: Option<usize>,
    /// 最大 Session 空闲超时时间
    /// 
    /// Maximum idle timeout for sessions
    max_session_idle_timeout: Option<u64>,
}

impl WebSocketProperties {
    /// 获取最大消息大小
    /// 
    /// Get the maximum message size
    pub fn max_msg_size(&self) -> Option<usize> {
        self.max_msg_size
    }

    /// 获取最大二写入 buff 容量大小
    /// 
    /// Get the maximum buffer capacity for writing
    pub fn max_write_buffer_size(&self) -> Option<usize> {
        self.max_write_buffer_size
    }

    /// 获取最大 Session 空闲超时时间
    /// 
    /// Get the maximum idle timeout for sessions
    pub fn max_session_idle_timeout(&self) -> Option<u64> {
        self.max_session_idle_timeout
    }
}