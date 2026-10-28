use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    extract::ws::{CloseFrame, Message},
    http::HeaderMap,
};
use flume::Sender;
use next_web_core::{Uuid, any::AnyValue};

/// Represents a WebSocket session.
///
/// This structure is used to manage and identify an individual WebSocket connection.
/// It contains the session ID, a channel sender for sending messages, and the remote address of the client.
///
/// # Fields
/// - `id`: Unique identifier for the session (UUID).
/// - `msg_channel`: Channel sender used to send WebSocket messages asynchronously.
/// - `remote_address`: The client's remote socket address.
/// - `header`: request header
/// - `path`: request path
/// - `attributes`: what the handshake interceptors collected, shared rather than
///   copied so that cloning a session stays cheap
///
#[derive(Clone)]
pub struct WebSocketSession {
    id: Uuid,
    msg_channel: Sender<Message>,
    remote_address: SocketAddr,
    header: HeaderMap,
    path: String,
    attributes: Arc<HashMap<String, AnyValue>>,
}

impl WebSocketSession {
    /// Creates a new WebSocket session with a unique ID.
    ///
    /// # Arguments
    /// - `msg_channel`: A sender channel used to send messages to the client.
    /// - `remote_address`: The client’s remote socket address.
    /// - `header`: request header
    /// - `path`: request path
    ///
    /// # Returns
    /// A new instance of `WebSocketSession`.
    ///
    pub fn new(
        msg_channel: Sender<Message>,
        remote_address: SocketAddr,
        header: HeaderMap,
        path: String,
    ) -> Self {
        Self::with_attributes(msg_channel, remote_address, header, path, HashMap::new())
    }

    /// Creates a new WebSocket session that also carries the attributes the
    /// handshake interceptors collected.
    ///
    /// # Arguments
    /// - `msg_channel`: A sender channel used to send messages to the client.
    /// - `remote_address`: The client’s remote socket address.
    /// - `header`: request header
    /// - `path`: request path
    /// - `attributes`: what the handshake interceptors put in the map they were
    ///   given
    ///
    /// # Returns
    /// A new instance of `WebSocketSession`.
    ///
    pub fn with_attributes(
        msg_channel: Sender<Message>,
        remote_address: SocketAddr,
        header: HeaderMap,
        path: String,
        attributes: HashMap<String, AnyValue>,
    ) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            msg_channel,
            remote_address,
            header,
            path,
            attributes: Arc::new(attributes),
        }
    }

    /// Sends a message through the WebSocket session.
    ///
    /// # Arguments
    /// - `message`: The WebSocket message to be sent.
    ///
    /// # Returns
    /// - `Ok(())` if the message was successfully sent.
    /// - `Err(Message)` if the send operation failed (e.g., channel closed).
    ///
    pub async fn send_message(&self, message: Message) -> Result<(), Message> {
        if let Err(error) = self.msg_channel.send_async(message).await {
            return Err(error.0);
        }
        Ok(())
    }

    /// Checks whether the WebSocket connection is still open.
    ///
    /// Currently, this method always returns `false`, indicating that the connection is considered closed.
    /// In a real implementation, this could involve checking the state of the underlying connection.
    ///
    /// # Returns
    /// A boolean indicating whether the session is active/open.
    ///
    pub async fn is_open(&self) -> bool {
        !self.msg_channel.is_disconnected()
    }

    /// Closes the WebSocket session gracefully.
    ///
    /// Sends a close message with a normal closure code and an empty reason.
    /// This signals the client that the server intends to close the connection.
    ///
    pub async fn close(&self) {
        let _ = self
            .msg_channel
            .send_async(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: "The server actively closes the connection.".into(),
            })))
            .await;
    }

    /// Gets the unique identifier of the session.
    ///
    /// # Returns
    /// A reference to the session's UUID.
    ///
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Gets the remote socket address of the connected client.
    ///
    /// # Returns
    /// A reference to the client's `SocketAddr`.
    ///
    pub fn remote_address(&self) -> &SocketAddr {
        &self.remote_address
    }

    /// Get the request header data of the currently connected client
    ///
    /// # Returns
    /// A reference to  `HeaderMap`.
    ///
    pub fn header(&self) -> &HeaderMap {
        &self.header
    }

    /// Get the request path of the current connected client
    ///
    /// # Returns
    /// A reference to  `String`.
    ///
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    /// Gets everything the handshake interceptors put in the attribute map.
    ///
    /// # Returns
    /// The attributes, keyed by the name the interceptor used.
    ///
    pub fn attributes(&self) -> &HashMap<String, AnyValue> {
        &self.attributes
    }

    /// Gets one attribute by name, for the common case of a single value such as
    /// the authenticated principal.
    ///
    /// # Arguments
    /// - `name`: The key an interceptor used.
    ///
    /// # Returns
    /// The value, or `None` when no interceptor set that name.
    ///
    pub fn attribute(&self, name: &str) -> Option<&AnyValue> {
        self.attributes.get(name)
    }
}
