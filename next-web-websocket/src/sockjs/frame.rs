//! SockJS frame types representing the protocol's message formats.
//!
//! SockJS uses a minimal framing protocol over HTTP to simulate bidirectional
//! communication when native WebSocket is unavailable.

/// Represents the different frame types in the SockJS protocol.
///
/// # Frame Format
///
/// - **Open** (`o`): Sent when a connection is established
/// - **Heartbeat** (`h`): Sent periodically to keep connection alive (default: 25s)
/// - **Message** (`a["msg1","msg2"]`): Array of application messages
/// - **Close** (`c[code,"reason"]`): Connection closure with status code
///
/// # Examples
///
/// ```rust
/// use next_web_websocket::sockjs::SockJsFrame;
///
/// // Open frame
/// let open = SockJsFrame::Open;
///
/// // Message frame with multiple messages
/// let messages = SockJsFrame::Message(vec![
///     "Hello".to_string(),
///     "World".to_string(),
/// ]);
///
/// // Close frame with code and reason
/// let close = SockJsFrame::Close {
///     code: 3000,
///     reason: "Go away!".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SockJsFrame {
    /// Open frame - indicates successful connection establishment.
    /// Encoded as: `o`
    Open,

    /// Heartbeat frame - keeps connection alive to prevent proxy timeout.
    /// Encoded as: `h`
    Heartbeat,

    /// Message frame - contains an array of application messages.
    /// Encoded as: `a["msg1","msg2",...]`
    Message(Vec<String>),

    /// Close frame - indicates connection closure with status code and reason.
    /// Encoded as: `c[code,"reason"]`
    Close { code: u16, reason: String },
}

impl SockJsFrame {
    /// Creates a close frame with standard code 3000 (normal closure).
    pub fn normal_close(reason: impl Into<String>) -> Self {
        Self::Close {
            code: 3000,
            reason: reason.into(),
        }
    }

    /// Creates a close frame with code 3001 (going away).
    pub fn going_away() -> Self {
        Self::Close {
            code: 3001,
            reason: "Going away".to_string(),
        }
    }

    /// Creates a close frame with code 3002 (protocol error).
    pub fn protocol_error(reason: impl Into<String>) -> Self {
        Self::Close {
            code: 3002,
            reason: reason.into(),
        }
    }

    /// Creates a close frame with code 3003 (unsupported data).
    pub fn unsupported_data(reason: impl Into<String>) -> Self {
        Self::Close {
            code: 3003,
            reason: reason.into(),
        }
    }
}
