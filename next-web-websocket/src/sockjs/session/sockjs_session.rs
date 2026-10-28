use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc, OnceLock, RwLock as StdRwLock,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Instant,
};

use axum::{
    extract::ws::{CloseFrame, Message},
    http::HeaderMap,
};
use next_web_core::{any::AnyValue, error::BoxError};
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

    /// What the handshake interceptors collected for the request that created
    /// this session.
    attributes: Arc<HashMap<String, AnyValue>>,

    /// Address the session belongs to, fixed by the request that created it.
    binding: OnceLock<IpAddr>,

    /// Largest number of frames the HTTP delivery queue may hold.
    ///
    /// Shared rather than copied, so a clone of the session — the output pump
    /// holds one — counts against the same limit.
    queue_capacity: Arc<AtomicUsize>,

    /// How many frames were dropped because that queue was full.
    dropped_messages: Arc<AtomicU64>,
}

/// Frames the HTTP delivery queue may hold before the oldest is dropped.
///
/// A delivery queue is a cache for the next poll, so it only grows while no
/// receiver is attached. The service sets it from
/// `http_message_cache_size`; this is the value used until then.
const DEFAULT_QUEUE_CAPACITY: usize = 100;

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
        Self::with_attributes(id, codec, remote_addr, headers, HashMap::new())
    }

    /// Creates a new SockJS session that also carries what the handshake
    /// interceptors collected.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique session identifier
    /// * `codec` - Message codec for frame encoding/decoding
    /// * `remote_addr` - Client's remote address
    /// * `headers` - HTTP headers from initial request
    /// * `attributes` - values an interceptor put in the map it was given; the
    ///   handler reads them from the session it is called with
    pub fn with_attributes(
        id: String,
        codec: Arc<dyn SockJsMessageCodec>,
        remote_addr: SocketAddr,
        headers: HeaderMap,
        attributes: HashMap<String, AnyValue>,
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
            attributes: Arc::new(attributes),
            binding: OnceLock::new(),
            queue_capacity: Arc::new(AtomicUsize::new(DEFAULT_QUEUE_CAPACITY)),
            dropped_messages: Arc::new(AtomicU64::new(0)),
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

    /// Returns what the handshake interceptors collected for this session.
    ///
    /// These are the same values the plain WebSocket path puts on a
    /// [`WebSocketSession`], so a handler sees the same attributes whichever
    /// transport carried the connection.
    pub fn attributes(&self) -> &HashMap<String, AnyValue> {
        &self.attributes
    }

    /// Returns when the session was created.
    pub fn created_at(&self) -> Instant {
        self.created_at
    }

    /// Attaches a WebSocket session to this SockJS session.
    ///
    /// Used when the client connects via native WebSocket transport.
    pub async fn attach_websocket(&self, ws_session: WebSocketSession) {
        let previous = {
            let mut guard = self.ws_session.write().await;
            let previous = guard.take();
            *guard = Some(ws_session);
            previous
        };

        // Close the socket this one replaced rather than dropping it on the floor:
        // the previous connection's receive loop would otherwise keep running with
        // nobody writing to it, and its client would never be told it lost the
        // session.
        if let Some(previous) = previous {
            tracing::debug!("A new WebSocket replaced one already attached to this session");
            previous.close().await;
        }

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
        let application_session = WebSocketSession::with_attributes(
            message_tx,
            self.remote_addr,
            self.headers.clone(),
            format!("/sockjs/{}", self.id),
            self.attributes.as_ref().clone(),
        );
        // The pump gets its own handle on the channel, so that waiting for a
        // message does not hold the lock the flush path needs.
        let pump_rx = message_rx.clone();
        let message_rx = Arc::new(Mutex::new(message_rx));
        lifecycle.opened = true;
        lifecycle.handler = Some(handler.clone());
        lifecycle.application_session = Some(application_session.clone());
        lifecycle.output_rx = Some(message_rx.clone());
        let (output_pump_stop, output_pump_stop_rx) = tokio::sync::watch::channel(false);
        lifecycle.output_pump_stop = Some(output_pump_stop);

        // Start the pump before invoking on_open so callbacks can enqueue any
        // number of messages without blocking on the bounded channel.
        self.spawn_application_output_pump(pump_rx, output_pump_stop_rx);
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
        // A session swept for inactivity, or a transport that simply went away,
        // never went through `close`, so record a close frame here too. Without it
        // a client still polling would be answered with an empty body and would
        // have to guess from the EOF that the session had ended.
        if http_state.close_frame.is_none() {
            http_state.close_frame = self.codec.encode(&SockJsFrame::going_away()).ok();
        }
        drop(http_state);
        if let (Some(handler), Some(application_session)) = (handler, application_session)
            && let Err(error) = handler.on_close(&application_session, close).await
        {
            let _ = handler.on_error(&application_session, error).await;
        }
        self.stop_application_output_pump().await;
    }

    fn spawn_application_output_pump(
        &self,
        pump_rx: flume::Receiver<Message>,
        mut stop_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        let session = self.clone();
        let output_lock = self.output_lock.clone();
        tokio::spawn(async move {
            loop {
                // This handle on the channel lets the pump *wait* for a message
                // without holding `output_lock`, which the flush path needs. A
                // `try_recv` behind that lock forced a poll every few milliseconds
                // for the whole life of the session — 200 wake-ups a second per
                // connection while idle. A flume clone shares the queue, so a
                // message still reaches exactly one of the two drainers, and
                // `output_lock` still serialises the forwarding itself.
                let message = tokio::select! {
                    changed = stop_rx.changed() => {
                        if changed.is_err() || *stop_rx.borrow() {
                            break;
                        }
                        continue;
                    }
                    received = pump_rx.recv_async() => match received {
                        Ok(message) => message,
                        // The application session is gone; nothing more will come.
                        Err(_) => break,
                    },
                };

                let _output_guard = output_lock.lock().await;
                if let Err(error) = session.forward_application_message(message).await {
                    tracing::debug!("Failed to forward SockJS application message: {error}");
                    break;
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
        while let Ok(message) = message_rx.lock().await.try_recv() {
            self.forward_application_message(message).await?;
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

        // Remember the close frame, so a poll that arrives after the queued copy
        // has already been delivered is still answered with `c[...]` rather than
        // an empty body.
        if matches!(frame, SockJsFrame::Close { .. }) {
            http_state.close_frame = Some(encoded.clone());
        }

        // Bounded. The queue only grows while no receiver is attached — a client
        // that vanished, or one still inside the disconnect window — so without a
        // cap an application that keeps sending would grow it without limit.
        let capacity = self.queue_capacity.load(Ordering::Relaxed).max(1);
        while http_state.message_queue.len() >= capacity {
            http_state.message_queue.pop_front();
            self.dropped_messages.fetch_add(1, Ordering::Relaxed);
        }

        http_state.message_queue.push_back(encoded);
        http_state.last_activity = Instant::now();

        Ok(())
    }

    /// Sets how many frames the HTTP delivery queue may hold.
    ///
    /// The service sets this from `http_message_cache_size` when it creates the
    /// session. Frames beyond it drop the oldest, because a delivery queue is a
    /// cache for the next poll and the newest frames are the ones a client that
    /// comes back can still use.
    pub fn set_queue_capacity(&self, capacity: usize) {
        self.queue_capacity.store(capacity.max(1), Ordering::Relaxed);
    }

    /// Returns how many frames have been dropped because the queue was full.
    ///
    /// Worth alerting on: a number that keeps rising means a client is not
    /// collecting its messages.
    pub fn dropped_message_count(&self) -> u64 {
        self.dropped_messages.load(Ordering::Relaxed)
    }

    /// Fixes the address this session belongs to.
    ///
    /// Only the first call has an effect, so a request that reaches an existing
    /// session cannot change whose it is. The **IP** is bound rather than the
    /// full address, because every request of a SockJS session arrives on a new
    /// TCP connection with a new source port.
    pub fn bind_to(&self, ip: IpAddr) {
        let _ = self.binding.set(ip);
    }

    /// Returns whether this session belongs to `ip`.
    ///
    /// A session with no binding belongs to anyone, which is the case for one
    /// created through [`crate::sockjs::service::SockJsService::create_session`]
    /// rather than by a request.
    pub fn belongs_to(&self, ip: IpAddr) -> bool {
        self.binding.get().is_none_or(|bound| *bound == ip)
    }

    /// Returns the address this session belongs to, if it has one.
    pub fn bound_ip(&self) -> Option<IpAddr> {
        self.binding.get().copied()
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

    /// Takes over as this session's receiver and returns the token proving it.
    ///
    /// SockJS wants exactly one receiver per session, and a second one normally
    /// means the client gave up on its previous request and started another — so
    /// the newest wins. Bumping the generation is what makes whatever was
    /// receiving end its response at its next poll; nothing is pushed to it,
    /// because the client has already stopped waiting on that request.
    pub async fn begin_receive(&self) -> u64 {
        let mut http_state = self.http_state.write().await;
        http_state.receiver_generation = http_state.receiver_generation.wrapping_add(1);
        http_state.receiver_generation
    }

    /// Drains queued frames for the receiver holding `generation`.
    ///
    /// Returns `None` once a newer receiver has taken the session over, and
    /// drains nothing in that case. The check and the drain share one lock on
    /// purpose: done separately, a receiver that had just been replaced could
    /// still take a frame the live one is waiting for.
    pub async fn poll_for_receiver(
        &self,
        generation: u64,
        max_messages: usize,
    ) -> Option<Vec<String>> {
        let mut http_state = self.http_state.write().await;
        if http_state.receiver_generation != generation {
            return None;
        }

        let count = std::cmp::min(max_messages, http_state.message_queue.len());
        Some(http_state.message_queue.drain(..count).collect())
    }

    /// Returns how many receive requests have taken this session over.
    pub async fn receiver_generation(&self) -> u64 {
        self.http_state.read().await.receiver_generation
    }

    /// Returns the close frame this session was closed with, if it was closed.
    ///
    /// Recorded when a close frame is sent and when the transport closes the
    /// session, so that a poll arriving afterwards can still report `c[...]`
    /// instead of an empty body.
    pub async fn close_frame(&self) -> Option<String> {
        self.http_state.read().await.close_frame.clone()
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
    use crate::{
        server::support::ws_session::WebSocketSession,
        sockjs::codec::JsonSockJsMessageCodec,
        ws_handler::{WSResult, WebSocketHandler},
    };
    use axum::extract::ws::{CloseFrame, Message};
    use next_web_core::async_trait;
    use std::{
        collections::BTreeSet,
        error::Error,
        net::{IpAddr, SocketAddr},
        sync::Arc,
    };

    #[tokio::test]
    async fn close_preserves_valid_codes_and_maps_reserved_codes() {
        let session = SockJsSession::new(
            "close-test".to_string(),
            Arc::new(JsonSockJsMessageCodec),
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

    /// Builds a session with no application handler, which is enough for
    /// everything that only touches the delivery queue or the binding.
    fn test_session(id: &str) -> SockJsSession {
        SockJsSession::new(
            id.to_string(),
            Arc::new(JsonSockJsMessageCodec),
            "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            axum::http::HeaderMap::new(),
        )
    }

    #[tokio::test]
    async fn the_delivery_queue_is_capped_and_keeps_the_newest_frames() {
        let session = test_session("queue-cap");
        session.set_queue_capacity(2);

        for index in 0..5 {
            session
                .send_messages(vec![format!("message-{index}")])
                .await
                .unwrap();
        }

        // Bounded, and what survives is what a client that comes back can still
        // use: the newest frames rather than the oldest.
        assert_eq!(session.queued_message_count().await, 2);
        assert_eq!(session.dropped_message_count(), 3);
        assert_eq!(
            session.drain_messages().await,
            vec!["a[\"message-3\"]".to_string(), "a[\"message-4\"]".to_string()]
        );
    }

    #[tokio::test]
    async fn a_session_belongs_to_the_address_that_first_used_it() {
        let session = test_session("binding");
        let first: IpAddr = "10.0.0.1".parse().unwrap();
        let other: IpAddr = "10.0.0.2".parse().unwrap();

        // Unbound until a request arrives, so a session that no request has
        // reached yet is not refused.
        assert_eq!(session.bound_ip(), None);
        assert!(session.belongs_to(other));

        session.bind_to(first);
        assert_eq!(session.bound_ip(), Some(first));
        assert!(session.belongs_to(first));
        assert!(!session.belongs_to(other));

        // A later caller cannot take the session over.
        session.bind_to(other);
        assert_eq!(session.bound_ip(), Some(first));
        assert!(!session.belongs_to(other));
    }

    #[tokio::test]
    async fn the_close_frame_survives_for_a_poll_that_arrives_later() {
        let session = test_session("close-frame");

        // `close` both queues the frame and records it.
        session.close(1000, "bye".to_string()).await.unwrap();
        assert_eq!(
            session.drain_messages().await,
            vec!["c[1000,\"bye\"]".to_string()]
        );

        // The queued copy has been delivered by that poll, but a later one can
        // still report the close frame rather than an empty body.
        assert_eq!(
            session.close_frame().await.as_deref(),
            Some("c[1000,\"bye\"]")
        );

        // A session closed without going through `close` — one the sweep reaped,
        // or a transport that simply went away — gets a frame too, so its next
        // poll answers with a close rather than a bare EOF.
        let swept = test_session("swept");
        swept.notify_close(None).await;
        assert!(swept.is_closed().await);
        assert!(swept.close_frame().await.is_some());
    }

    #[tokio::test]
    async fn a_newer_receiver_takes_the_session_over_and_the_old_one_stops() {
        let session = test_session("receiver");

        let first = session.begin_receive().await;
        session
            .send_messages(vec!["for-the-first".to_string()])
            .await
            .unwrap();
        assert_eq!(
            session.poll_for_receiver(first, 10).await,
            Some(vec!["a[\"for-the-first\"]".to_string()])
        );

        // A second request takes over, as a client that gave up on its previous
        // poll and started another would.
        let second = session.begin_receive().await;
        assert_ne!(first, second);

        session
            .send_messages(vec!["for-the-second".to_string()])
            .await
            .unwrap();

        // The replaced receiver drains nothing, so it cannot take a frame the live
        // receiver is waiting for. This is why the check and the drain share a
        // lock rather than being separate calls.
        assert_eq!(session.poll_for_receiver(first, 10).await, None);
        assert_eq!(
            session.poll_for_receiver(second, 10).await,
            Some(vec!["a[\"for-the-second\"]".to_string()])
        );
        // Exactly one receiver got it.
        assert_eq!(session.queued_message_count().await, 0);
    }

    #[tokio::test]
    async fn concurrent_senders_are_either_queued_or_counted_as_dropped() {
        const SENDERS: usize = 8;
        const EACH: usize = 50;
        const CAPACITY: usize = 16;

        let session = Arc::new(test_session("cap-concurrent"));
        session.set_queue_capacity(CAPACITY);

        let mut tasks = Vec::new();
        for sender in 0..SENDERS {
            let session = Arc::clone(&session);
            tasks.push(tokio::spawn(async move {
                for index in 0..EACH {
                    session
                        .send_messages(vec![format!("{sender}-{index}")])
                        .await
                        .expect("the queue accepts a frame");
                }
            }));
        }
        for task in tasks {
            task.await.expect("no sender panicked");
        }

        // Every frame is accounted for as either still queued or explicitly
        // dropped, whatever the interleaving was, and the queue never grew past
        // its cap. This is the property that makes the cap a bound rather than a
        // race.
        let queued = session.queued_message_count().await as u64;
        let dropped = session.dropped_message_count();
        assert_eq!(
            queued + dropped,
            (SENDERS * EACH) as u64,
            "queued {queued} + dropped {dropped} should account for {} frames",
            SENDERS * EACH
        );
        assert!(queued <= CAPACITY as u64, "the queue grew past its cap");
    }

    #[tokio::test]
    async fn concurrent_receivers_never_see_the_same_frame_twice() {
        const FRAMES: usize = 200;
        const RECEIVERS: usize = 8;

        let session = Arc::new(test_session("receivers-concurrent"));
        session.set_queue_capacity(FRAMES * 2);
        for index in 0..FRAMES {
            session
                .send_messages(vec![format!("frame-{index}")])
                .await
                .unwrap();
        }

        let mut tasks = Vec::new();
        for _ in 0..RECEIVERS {
            let session = Arc::clone(&session);
            tasks.push(tokio::spawn(async move {
                let generation = session.begin_receive().await;
                let mut collected = Vec::new();
                // Keep asking while this receiver is still the current one; a
                // later one stops it, and an empty answer means the queue ran dry.
                while let Some(frames) = session.poll_for_receiver(generation, 8).await {
                    if frames.is_empty() {
                        break;
                    }
                    collected.extend(frames);
                }
                collected
            }));
        }

        let mut delivered = Vec::new();
        for task in tasks {
            delivered.extend(task.await.expect("no receiver panicked"));
        }

        // No frame reached two receivers, whichever order they ran in.
        let unique = delivered.iter().collect::<BTreeSet<_>>();
        assert_eq!(
            unique.len(),
            delivered.len(),
            "a frame was handed to more than one receiver"
        );

        // And nothing vanished: what the receivers took plus what is left is
        // everything that was sent.
        assert_eq!(delivered.len() + session.queued_message_count().await, FRAMES);
    }

    /// Sends from `on_open`, so the pump has to keep up with a producer rather
    /// than draining a channel that was already full.
    struct BurstHandler {
        messages: usize,
    }

    #[async_trait]
    impl WebSocketHandler for BurstHandler {
        async fn on_open(&self, session: &WebSocketSession) -> WSResult<()> {
            for index in 0..self.messages {
                session
                    .send_message(Message::Text(format!("burst-{index}").into()))
                    .await
                    .map_err(|error| format!("failed to queue burst {index}: {error:?}"))?;
            }
            Ok(())
        }

        async fn on_message(&self, _session: &WebSocketSession, _message: Message) -> WSResult<()> {
            Ok(())
        }

        async fn on_error(
            &self,
            _session: &WebSocketSession,
            _error: Box<dyn Error + Send + Sync>,
        ) -> WSResult<()> {
            Ok(())
        }

        async fn on_close(
            &self,
            _session: &WebSocketSession,
            _close: Option<CloseFrame>,
        ) -> WSResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn the_output_pump_forwards_each_message_exactly_once() {
        // Above the application channel's own capacity, so the producer has to be
        // interleaved with the pump instead of the channel swallowing the lot.
        const MESSAGES: usize = 300;

        let session = test_session("pump");
        // Above the delivery queue's cap too, so a count short of the total can
        // only mean the pump lost something.
        session.set_queue_capacity(MESSAGES * 2);
        session.set_websocket_handler(Arc::new(BurstHandler {
            messages: MESSAGES,
        }));

        session.ensure_handler().await.expect("the handler is opened");

        // The pump forwards asynchronously, so let the queue settle.
        let mut queued = 0;
        for _ in 0..200 {
            queued = session.queued_message_count().await;
            if queued >= MESSAGES {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert_eq!(queued, MESSAGES, "the pump lost or duplicated frames");
        assert_eq!(session.dropped_message_count(), 0);

        // Each message arrived as its own `a[...]` frame, exactly once.
        let mut seen = BTreeSet::new();
        for frame in session.drain_messages().await {
            let payload = frame.strip_prefix('a').expect("an a[...] frame");
            let texts: Vec<String> =
                serde_json::from_str(payload).expect("the payload is a JSON array");
            for text in texts {
                assert!(seen.insert(text.clone()), "delivered twice: {text}");
            }
        }

        assert_eq!(seen.len(), MESSAGES);
        for index in 0..MESSAGES {
            assert!(
                seen.contains(&format!("burst-{index}")),
                "lost burst-{index}"
            );
        }
    }
}
