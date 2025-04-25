use rudi::{Properties, Singleton};

/// WebSocket配置属性，用于配置WebSocket连接的相关参数
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.mqtt")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct WebSocketProperties {
    max_msg_size: Option<usize>,
    max_write_buffer_size: Option<usize>,
    max_session_idle_timeout: Option<u64>,
}

impl WebSocketProperties {
    /// 最大消息大小
    pub fn max_msg_size(&self) -> Option<usize> {
        self.max_msg_size
    }

    /// 最大二写入 buff 容量大小
    pub fn max_write_buffer_size(&self) -> Option<usize> {
        self.max_write_buffer_size
    }

    /// 最大 Session 空闲超时时间
    pub fn max_session_idle_timeout(&self) -> Option<u64> {
        self.max_session_idle_timeout
    }
}