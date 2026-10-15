use next_web_macros::configuration_properties;

/// WebSocket configuration properties, used to configure parameters related to WebSocket connections.
#[configuration_properties(prefix = "next.ws")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct WebSocketProperties {
    /// Maximum message size.
    max_msg_size: Option<usize>,

    /// Maximum buffer capacity for writing.
    max_write_buffer_size: Option<usize>,

    /// How long a client may stay silent before the server closes its
    /// connection, in seconds.
    ///
    /// Left unset, or set to `0`, the connection stays open until the client
    /// closes it or the operating system gives up on it.
    max_session_idle_timeout: Option<u64>,

    /// WebSocket message channel capacity.
    msg_channel_capacity: Option<u32>,
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
    pub fn msg_channel_capacity(&self) -> Option<u32> {
        self.msg_channel_capacity
    }

    /// Sets the `max_msg_size` field.
    pub fn set_max_msg_size<V>(&mut self, size: V)
    where
        V: Into<Option<usize>>,
    {
        self.max_msg_size = size.into();
    }

    /// Sets the `max_write_buffer_size` field.
    pub fn set_max_write_buffer_size<V>(&mut self, size: V)
    where
        V: Into<Option<usize>>,
    {
        self.max_write_buffer_size = size.into();
    }

    /// Sets the `max_session_idle_timeout` field.
    pub fn set_max_session_idle_timeout<V>(&mut self, timeout: V)
    where
        V: Into<Option<u64>>,
    {
        self.max_session_idle_timeout = timeout.into();
    }

    /// Sets the `msg_channel_capacity` field.
    pub fn set_msg_channel_capacity(&mut self, cap: u32) {
        self.msg_channel_capacity = Some(cap);
    }
}
