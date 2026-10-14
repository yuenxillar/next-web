use std::{
    net::SocketAddr,
    sync::{Arc, RwLock as StdRwLock},
    time::Instant,
};

use axum::{
    extract::ws::{CloseFrame, Message},
    http::HeaderMap,
};
use next_web_core::error::BoxError;
use tokio::sync::{Mutex, RwLock};

use crate::{
    server::support::ws_session::WebSocketSession,
    sockjs::{SockJsFrame, codec::SockJsMessageCodec, transport::TransportType},
    ws_handler::WebSocketHandler,
};

use super::HttpSessionState;

/// A SockJS session that can work with both WebSocket and HTTP transports.
///
/// This session maintains state across multiple HTTP requests and supports
/// switching between transport mechanisms. It wraps an optional `WebSocketSession`
/// for native WebSocket transport and maintains HTTP state for fallback transports.
#[derive(Clone)]
pub struct SockJsSession {
    /// Unique SockJS session identifier (not the same as WebSocket UUID).
    id: String,

    /// Optional WebSocket session (only present for WebSocket transport).
    ws_session: Arc<RwLock<Option<WebSocketSession>>>,

    /// State for HTTP-based transports.
    http_state: Arc<RwLock<HttpSessionState>>,

    /// Message codec for encoding/decoding frames.
    codec: Arc<dyn SockJsMessageCodec>,

    /// Remote address of the client.
    remote_addr: SocketAddr,

    /// HTTP headers from the initial request.
    headers: HeaderMap,

    /// Application handler bound by the `with_sock_js()` registration.
    application_handler: Arc<StdRwLock<Option<Arc<dyn WebSocketHandler>>>>,

    /// Application callback/session lifecycle, initialized on first transport use.
    handler_lifecycle: Arc<Mutex<HandlerLifecycle>>,

    /// Serializes application-output draining and the background output pump.
    output_lock: Arc<Mutex<()>>,

    /// Time when the session was created.
    created_at: Instant,
}

impl SockJsSession {
    /// Creates a new SockJS session.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique session identifier
    /// * `codec` - Message codec for frame encoding/decoding
    /// * `remote_addr` - Client's remote address
    /// * `headers` - HTTP headers from initial request
    pub fn new(
        id: String,
        codec: Arc<dyn SockJsMessageCodec>,
        remote_addr: SocketAddr,
        headers: HeaderMap,
    ) -> Self {
        Self {
            id,
            ws_session: Arc::new(RwLock::new(None)),
            http_state: Arc::new(RwLock::new(HttpSessionState::default())),
            codec,
            remote_addr,
            headers,
            application_handler: Arc::new(StdRwLock::new(None)),
            handler_lifecycle: Arc::new(Mutex::new(HandlerLifecycle::default())),
            output_lock: Arc::new(Mutex::new(())),
            created_at: Instant::now(),
        }
    }

    /// Binds the application WebSocket handler to this SockJS session.
    pub fn set_websocket_handler(&self, handler: Arc<dyn WebSocketHandler>) {
        *self
            .application_handler
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(handler);
    }

    /// Returns the session ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the remote address.
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Returns the HTTP headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns when the session was created.
    pub fn created_at(&self) -> Instant {
        self.created_at
    }

    /// Attaches a WebSocket session to this SockJS session.
    ///
    /// Used when the client connects via native WebSocket transport.
    pub async fn attach_websocket(&self, ws_session: WebSocketSession) {
        let mut guard = self.ws_session.write().await;
        *guard = Some(ws_session);

        // The transport sets its own open frame after attaching the socket.
        let mut http_state = self.http_state.write().await;
        http_state.transport_type = Some(TransportType::WebSocket);
    }

    /// Removes the active native WebSocket after its transport closes.
    pub async fn detach_websocket(&self) {
        *self.ws_session.write().await = None;
    }

    /// Opens the application callback session once for this SockJS session.
    pub async fn ensure_handler(&self) -> Result<(), BoxError> {
        let handler = self
            .application_handler
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let Some(handler) = handler else {
            return Ok(());
        };

        let mut lifecycle = self.handler_lifecycle.lock().await;
        if lifecycle.opened {
            drop(lifecycle);
            return self.flush_application_output().await;
        }

        let (message_tx, message_rx) = flume::bounded(128);
        let application_session = WebSocketSession::new(
            message_tx,
            self.remote_addr,
            self.headers.clone(),
            format!("/sockjs/{}", self.id),
        );
        let message_rx = Arc::new(Mutex::new(message_rx));
        lifecycle.opened = true;
        lifecycle.handler = Some(handler.clone());
        lifecycle.application_session = Some(application_session.clone());
        lifecycle.output_rx = Some(message_rx.clone());
        let (output_pump_stop, output_pump_stop_rx) = tokio::sync::watch::channel(false);
        lifecycle.output_pump_stop = Some(output_pump_stop);

        // Start the pump before invoking on_open so callbacks can enqueue any
        // number of messages without blocking on the bounded channel.
        self.spawn_application_output_pump(message_rx, output_pump_stop_rx);
        let open_result = handler.on_open(&application_session).await;
        drop(lifecycle);

        if let Err(error) = open_result {
            let message = error.to_string();
            let _ = handler.on_error(&application_session, error).await;
            self.flush_application_output().await?;
            return Err(message.into());
        }

        self.flush_application_output().await
    }

    /// Dispatches one decoded SockJS client message to the application handler.
    pub async fn dispatch_client_text(&self, text: String) -> Result<(), BoxError> {
        self.ensure_handler().await?;

        let (handler, application_session) = {
            let lifecycle = self.handler_lifecycle.lock().await;
            match (
                lifecycle.handler.clone(),
                lifecycle.application_session.clone(),
            ) {
                (Some(handler), Some(session)) if !lifecycle.closed => (handler, session),
                _ => return Ok(()),
            }
        };

        if let Err(error) = handler
            .on_message(&application_session, Message::Text(text.into()))
            .await
        {
            let message = error.to_string();
            let _ = handler.on_error(&application_session, error).await;
            self.flush_application_output().await?;
            return Err(message.into());
        }

        self.flush_application_output().await
    }

    /// Notifies the application handler when the SockJS transport closes.
    pub async fn notify_close(&self, close: Option<CloseFrame>) {
        let (handler, application_session) = {
            let mut lifecycle = self.handler_lifecycle.lock().await;
            if lifecycle.closed {
                return;
            }
            lifecycle.closed = true;
            (
                lifecycle.handler.clone(),
                lifecycle.application_session.clone(),
            )
        };

        let mut http_state = self.http_state.write().await;
        http_state.closed = true;
        drop(http_state);
        if let (Some(handler), Some(application_session)) = (handler, application_session) {
            if let Err(error) = handler.on_close(&application_session, close).await {
                let _ = handler.on_error(&application_session, error).await;
            }
        }
        self.stop_application_output_pump().await;
    }

    fn spawn_application_output_pump(
        &self,
        message_rx: Arc<Mutex<flume::Receiver<Message>>>,
        mut stop_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        let session = self.clone();
        let output_lock = self.output_lock.clone();
        tokio::spawn(async move {
            loop {
                let _output_guard = output_lock.lock().await;
                let message = message_rx.lock().await.try_recv();
                match message {
                    Ok(message) => {
                        if let Err(error) = session.forward_application_message(message).await {
                            tracing::debug!(
                                "Failed to forward SockJS application message: {error}"
                            );
                            break;
                        }
                    }
                    Err(flume::TryRecvError::Empty) => {
                        drop(_output_guard);
                        tokio::select! {
                            _ = stop_rx.changed() => break,
                            _ = tokio::time::sleep(std::time::Duration::from_millis(5)) => {}
                        }
                    }
                    Err(flume::TryRecvError::Disconnected) => break,
                }
            }
        });
    }

    async fn flush_application_output(&self) -> Result<(), BoxError> {
        let message_rx = self.handler_lifecycle.lock().await.output_rx.clone();
        let Some(message_rx) = message_rx else {
            return Ok(());
        };

        let _output_guard = self.output_lock.lock().await;
        loop {
            match message_rx.lock().await.try_recv() {
                Ok(message) => self.forward_application_message(message).await?,
                Err(flume::TryRecvError::Empty | flume::TryRecvError::Disconnected) => break,
            }
        }
        Ok(())
    }

    async fn forward_application_message(&self, message: Message) -> Result<(), BoxError> {
        match message {
            Message::Text(text) => self.send_messages(vec![text.to_string()]).await,
            Message::Close(frame) => match frame {
                Some(frame) => self.close(frame.code, frame.reason.to_string()).await,
                None => self.close(1000, String::new()).await,
            },
            Message::Binary(_) | Message::Ping(_) | Message::Pong(_) => Ok(()),
        }
    }

    async fn stop_application_output_pump(&self) {
        if let Some(stop) = self.handler_lifecycle.lock().await.output_pump_stop.take() {
            let _ = stop.send(true);
        }
    }

    /// Sets the current transport type.
    pub async fn set_transport(&self, transport: TransportType) {
        let mut http_state = self.http_state.write().await;
        http_state.transport_type = Some(transport);
    }

    /// Sends a frame to the client.
    ///
    /// For WebSocket transport, sends directly through the WebSocket.
    /// For HTTP transports, queues the message for the next poll/stream request.
    pub async fn send_frame(&self, frame: SockJsFrame) -> Result<(), BoxError> {
        // Try WebSocket first
        if let Some(ws) = self.ws_session.read().await.as_ref() {
            let encoded = self.codec.encode(&frame).map_err(|e| e.to_string())?;
            ws.send_message(axum::extract::ws::Message::Text(encoded.into()))
                .await
                .map_err(|_| "WebSocket send failed")?;
            self.touch().await;
            return Ok(());
        }

        // Fall back to HTTP queue
        let encoded = self.codec.encode(&frame).map_err(|e| e.to_string())?;
        let mut http_state = self.http_state.write().await;
        http_state.message_queue.push_back(encoded);
        http_state.last_activity = Instant::now();

        Ok(())
    }

    /// Sends the open frame ('o') to the client.
    pub async fn send_open(&self) -> Result<(), BoxError> {
        let mut http_state = self.http_state.write().await;
        if http_state.opened {
            return Err("Session already opened".into());
        }
        http_state.opened = true;
        drop(http_state);

        self.send_frame(SockJsFrame::Open).await
    }

    /// Sends a heartbeat frame ('h') to the client.
    pub async fn send_heartbeat(&self) -> Result<(), BoxError> {
        self.send_frame(SockJsFrame::Heartbeat).await
    }

    /// Sends application messages to the client.
    pub async fn send_messages(&self, messages: Vec<String>) -> Result<(), BoxError> {
        if messages.is_empty() {
            return Ok(());
        }
        self.send_frame(SockJsFrame::Message(messages)).await
    }

    /// Closes the session with a close frame.
    pub async fn close(&self, code: u16, reason: String) -> Result<(), BoxError> {
        let code = normalize_sockjs_close_code(code);
        let mut http_state = self.http_state.write().await;
        http_state.closed = true;
        drop(http_state);

        self.send_frame(SockJsFrame::Close { code, reason }).await
    }

    /// Checks if the session is connected.
    pub async fn is_connected(&self) -> bool {
        // Check WebSocket
        if let Some(ws) = self.ws_session.read().await.as_ref() {
            return ws.is_open().await;
        }

        // Check HTTP state
        let http_state = self.http_state.read().await;
        !http_state.closed
    }

    /// Checks if the session has been opened.
    pub async fn is_opened(&self) -> bool {
        self.http_state.read().await.opened
    }

    /// Checks if the session is closed.
    pub async fn is_closed(&self) -> bool {
        self.http_state.read().await.closed
    }

    /// Returns queued messages for HTTP transports.
    ///
    /// This is used by polling and streaming transports to get messages
    /// waiting for delivery.
    pub async fn poll_messages(&self, max_messages: usize) -> Vec<String> {
        let mut http_state = self.http_state.write().await;
        let count = std::cmp::min(max_messages, http_state.message_queue.len());
        http_state.message_queue.drain(..count).collect()
    }

    /// Returns all queued messages.
    pub async fn drain_messages(&self) -> Vec<String> {
        let mut http_state = self.http_state.write().await;
        http_state.message_queue.drain(..).collect()
    }

    /// Updates the last activity timestamp.
    pub async fn touch(&self) {
        let mut http_state = self.http_state.write().await;
        http_state.last_activity = Instant::now();
    }

    /// Returns the last activity time.
    pub async fn last_activity(&self) -> Instant {
        self.http_state.read().await.last_activity
    }

    /// Checks if the session has expired based on disconnect delay.
    pub async fn is_expired(&self, disconnect_delay_ms: u64) -> bool {
        // A live native WebSocket can remain idle longer than the HTTP
        // disconnect delay. Do not evict it while its sender is still open.
        let ws_session = self.ws_session.read().await.clone();
        if let Some(ws_session) = ws_session
            && ws_session.is_open().await
        {
            return false;
        }

        let http_state = self.http_state.read().await;
        let elapsed = http_state.last_activity.elapsed().as_millis() as u64;
        elapsed > disconnect_delay_ms
    }

    /// Returns the number of queued messages.
    pub async fn queued_message_count(&self) -> usize {
        self.http_state.read().await.message_queue.len()
    }

    /// Increments bytes sent counter (for streaming transport).
    pub async fn add_bytes_sent(&self, bytes: usize) {
        let mut http_state = self.http_state.write().await;
        http_state.bytes_sent += bytes;
    }

    /// Returns total bytes sent in current stream.
    pub async fn bytes_sent(&self) -> usize {
        self.http_state.read().await.bytes_sent
    }

    /// Resets bytes sent counter (when recycling streaming connection).
    pub async fn reset_bytes_sent(&self) {
        let mut http_state = self.http_state.write().await;
        http_state.bytes_sent = 0;
    }

    /// Returns the current transport type.
    pub async fn transport_type(&self) -> Option<TransportType> {
        self.http_state.read().await.transport_type
    }
}

#[derive(Default)]
struct HandlerLifecycle {
    opened: bool,
    closed: bool,
    handler: Option<Arc<dyn WebSocketHandler>>,
    application_session: Option<WebSocketSession>,
    output_rx: Option<Arc<Mutex<flume::Receiver<Message>>>>,
    output_pump_stop: Option<tokio::sync::watch::Sender<bool>>,
}

fn normalize_sockjs_close_code(code: u16) -> u16 {
    // Keep standard WebSocket statuses and application codes. Control and
    // unassigned codes must not be emitted as close frames.
    match code {
        1000..=1003 | 1007..=1014 | 3000..=4999 => code,
        _ => 3000,
    }
}

#[cfg(test)]
mod tests {
    use super::{SockJsSession, normalize_sockjs_close_code};
    use crate::sockjs::codec::Jackson2SockJsMessageCodec;
    use std::{net::SocketAddr, sync::Arc};

    #[tokio::test]
    async fn close_preserves_valid_codes_and_maps_reserved_codes() {
        let session = SockJsSession::new(
            "close-test".to_string(),
            Arc::new(Jackson2SockJsMessageCodec),
            "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            axum::http::HeaderMap::new(),
        );

        session.close(1000, "normal".to_string()).await.unwrap();

        assert_eq!(
            session.drain_messages().await,
            vec!["c[1000,\"normal\"]".to_string()]
        );

        assert_eq!(normalize_sockjs_close_code(1005), 3000);
        assert_eq!(normalize_sockjs_close_code(3001), 3001);
    }
}
