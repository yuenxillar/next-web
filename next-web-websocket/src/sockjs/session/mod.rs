//! SockJS session management.
//!
//! Sessions maintain the state of a SockJS connection across multiple
//! HTTP requests and can switch between different transport mechanisms.

use std::{collections::VecDeque, time::Instant};

use crate::sockjs::transport::TransportType;

/// SockJS session implementation.
pub mod sockjs_session;

pub use sockjs_session::SockJsSession;

/// State for HTTP-based transports (polling, streaming).
#[derive(Debug)]
pub(crate) struct HttpSessionState {
    /// Queue of messages waiting to be delivered to client.
    pub message_queue: VecDeque<String>,
    /// Last time any activity occurred on this session.
    pub last_activity: Instant,
    /// Current transport type being used.
    pub transport_type: Option<TransportType>,
    /// Total bytes sent in current streaming connection.
    pub bytes_sent: usize,
    /// Whether the session has been opened (sent 'o' frame).
    pub opened: bool,
    /// Whether the session has been closed.
    pub closed: bool,
}

impl Default for HttpSessionState {
    fn default() -> Self {
        Self {
            message_queue: VecDeque::new(),
            last_activity: Instant::now(),
            transport_type: None,
            bytes_sent: 0,
            opened: false,
            closed: false,
        }
    }
}
