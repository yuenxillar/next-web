use next_web_macros::properties;
use rudi_dev::singleton;

/// WebSocket configuration properties, used to configure parameters related to WebSocket connections
///
/// WebSocket配置属性，用于配置WebSocket连接的相关参数
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.ws")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct WebSocketProperties {
    /// Maximum message size
    ///
    /// 最大消息大小
    max_msg_size: Option<usize>,

    /// Maximum buffer capacity for writing
    ///
    /// 最大写入 buff 容量大小
    max_write_buffer_size: Option<usize>,

    /// Maximum idle timeout for sessions
    ///
    /// 最大 Session 空闲超时时间
    max_session_idle_timeout: Option<u64>,

    /// WebSocket message channel capacity
    ///
    /// WebSocket消息通道容量
    msg_channel_capacity: Option<i32>,
}

impl WebSocketProperties {
    /// Returns the `max_msg_size` field.
    pub fn max_msg_size(&self) -> Option<usize> {
        self.max_msg_size
    }

    /// Returns the `max_write_buffer_size` field.
    pub fn max_write_buffer_size(&self) -> Option<usize> {
        self.max_write_buffer_size
    }

    /// Returns the `max_session_idle_timeout` field.
    pub fn max_session_idle_timeout(&self) -> Option<u64> {
        self.max_session_idle_timeout
    }

    /// Returns the `msg_channel_capacity` field.
    pub fn msg_channel_capacity(&self) -> Option<i32> {
        self.msg_channel_capacity
    }

    /// Sets the `max_msg_size` field.
    pub fn set_max_msg_size<V>(&mut self, value: V)
    where
        V: Into<Option<usize>>,
    {
        self.max_msg_size = value.into();
    }

    /// Sets the `max_write_buffer_size` field.
    pub fn set_max_write_buffer_size<V>(&mut self, value: V)
    where
        V: Into<Option<usize>>,
    {
        self.max_write_buffer_size = value.into();
    }

    /// Sets the `max_session_idle_timeout` field.
    pub fn set_max_session_idle_timeout<V>(&mut self, value: V)
    where
        V: Into<Option<u64>>,
    {
        self.max_session_idle_timeout = value.into();
    }

    /// Sets the `msg_channel_capacity` field.
    pub fn set_msg_channel_capacity<V>(&mut self, value: V)
    where
        V: Into<Option<i32>>,
    {
        self.msg_channel_capacity = value.into();
    }
}
