use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use crate::server::support::{ws_context::WebSocketContext, ws_session::WebSocketSession};
use axum::{
    body::Bytes,
    extract::{
        ConnectInfo, Request, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use futures::{SinkExt, stream::StreamExt};
use next_web_core::any::AnyValue;
use tokio::sync::watch;
use tracing::{debug, error, info};

/// The handler for the HTTP request (this gets called when the HTTP request lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
///
/// WebSocket upgrade handler that processes HTTP to WebSocket protocol upgrade
///
/// # Parameters
/// - `ws`: WebSocket upgrade request
/// - `ctx`: WebSocket context with handlers and configurations
/// - `addr`: Client address
/// - `req`: The upgrade request, which an interceptor may rewrite
///
/// # Returns
/// - `Response`  implementing `IntoResponse`
pub(crate) async fn websocket_handle(
    ws: WebSocketUpgrade,
    State(ctx): State<Arc<WebSocketContext>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request,
) -> Response {
    let path = req.uri().path().to_owned();

    // A refused handshake deserves a status that says so. An empty response is
    // `200 OK`, which tells a client its request succeeded when the connection it
    // asked for was never established. An interceptor can still choose another
    // status with `set_status_code`, and the origin interceptor picks 403 itself.
    let mut attributes = HashMap::new();
    let mut refusal = StatusCode::FORBIDDEN.into_response();
    // Remember what the empty refusal holds, so that later only what an
    // interceptor actually added is carried onto the handshake response.
    let initial_refusal_headers = refusal.headers().clone();

    for (index, interceptor) in ctx.interceptors().iter().enumerate() {
        match interceptor
            .before_handshake(&mut req, &mut refusal, &mut attributes)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                debug!(client = %addr, %path, interceptor = index, "handshake refused");
                return refusal;
            }
            Err(error) => {
                // Failing closed is right; failing silently is not. Without this
                // an authentication service that is down looks exactly like a
                // client whose credentials were rejected.
                error!(
                    client = %addr,
                    %path,
                    interceptor = index,
                    "handshake interceptor failed, refusing the connection: {error}"
                );
                return refusal;
            }
        }
    }

    let properties = ctx.properties();
    let upgrade = ws
        .max_message_size(properties.max_msg_size().unwrap_or(64 << 20))
        .max_write_buffer_size(properties.max_write_buffer_size().unwrap_or(usize::MAX))
        .on_upgrade(move |socket| handle_socket(socket, ctx, addr, req, attributes))
        .into_response();

    carry_over_headers(&refusal, &initial_refusal_headers, upgrade)
}

/// Copies the headers an accepting interceptor added onto the handshake response.
///
/// An interceptor's `response` used to be sent only when it refused, so a header
/// written by an interceptor that went on to accept the connection reached
/// nobody — which is not what the trait says the parameter is for. Only the
/// difference from the empty refusal is carried, so the framing headers of that
/// empty response cannot end up on a `101`; and only headers the upgrade does not
/// set itself, so an interceptor cannot break the handshake by naming one of its
/// protocol headers. The interceptor's status is deliberately ignored: the
/// handshake is being accepted.
fn carry_over_headers(
    interceptor_response: &Response,
    initial_refusal_headers: &HeaderMap,
    mut upgrade: Response,
) -> Response {
    let additions: Vec<(HeaderName, HeaderValue)> = interceptor_response
        .headers()
        .iter()
        .filter(|(name, value)| initial_refusal_headers.get(*name) != Some(*value))
        .filter(|(name, _)| !upgrade.headers().contains_key(*name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();

    for (name, value) in additions {
        upgrade.headers_mut().append(name, value);
    }

    upgrade
}

/// Main function for handling WebSocket connections - one instance per connection
///
/// Handles the entire lifecycle of WebSocket connections including:
/// 1. Path matching handler
/// 2. Connection testing (Ping/Pong)
/// 3. Message sending/receiving
/// 4. Error handling and connection closure
///
/// # Parameters
/// - `socket`: WebSocket connection object
/// - `ctx`:    WebSocket context with handler registry
/// - `remote_address`: The client's address
/// - `req`:    HTTP request object
/// - `attributes`: What the handshake interceptors collected, handed to the
///   session so a handler can read it
pub(crate) async fn handle_socket(
    mut socket: WebSocket,
    ctx: Arc<WebSocketContext>,
    remote_address: SocketAddr,
    req: Request,
    attributes: HashMap<String, AnyValue>,
) {
    let path = req.uri().path().to_owned();
    debug!("Start processing WebSocket connections: {remote_address}, Path: {path}");

    // Match the corresponding handler through path matching
    // If no matching handler is found, close the connection.
    //
    // Looked up before the connection probe below: a path that answers nothing
    // should cost neither a write nor a channel.
    let handler = match ctx.get_handler(&path) {
        Some(handler) => handler,
        None => {
            error!("Path not found {path} The corresponding processor's connection will be closed");
            let _ = socket.close().await;
            return;
        }
    };

    // Send a ping packet to test connection
    // If sending fails, the connection may have been disconnected
    if socket
        .send(Message::Ping(Bytes::from_static(&[1])))
        .await
        .is_err()
    {
        error!(
            "Unable to send ping packet to client {remote_address}, The connection may have been disconnected"
        );
        return;
    }

    #[rustfmt::skip]
    let (msg_sender, msg_receiver) = ctx
        .properties()
        .msg_channel_capacity()
        .map(|cap| if cap != 0 { flume::bounded(cap as usize) } else { flume::unbounded() })
        .unwrap_or(flume::bounded(128));

    // How long the client may stay silent before the connection is closed. `None`
    // means the limit is off, which is the default; a configured `0` is treated
    // as off too, so that the property cannot be read as "close immediately".
    let idle_timeout = ctx
        .properties()
        .max_session_idle_timeout()
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs);

    let header = req.headers().clone();
    let session = WebSocketSession::with_attributes(
        msg_sender,
        remote_address,
        header,
        path.clone(),
        attributes,
    );

    // Split socket for simultaneous send/receive
    let (mut stream_sender, mut stream_receiver) = socket.split();

    // Send messages to client
    //
    // Spawned *before* `on_open`, not after it. A handler that greets the client
    // from `on_open` — as the crate's own SockJS example does — would otherwise
    // fill the outgoing channel while nothing drains it, and `send_message` would
    // wait for room that can never appear.
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let (sender_done_tx, mut sender_done_rx) = watch::channel(false);
    let sender_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // `changed` also reports an error when the sender is dropped, and
                // both outcomes mean this task should stop. Checking the value
                // rather than treating every wake-up as a shutdown keeps a
                // repeated send from ending the task early, and breaking on the
                // error is what stops a dropped connection from spinning here.
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        // Deliver what is already queued before stopping. Without
                        // this a frame the handler sent just before closing — the
                        // `Close` of an idle timeout, for instance — could be
                        // dropped, because once the shutdown is signalled the
                        // queue and the signal are both ready and `select!` picks
                        // either one.
                        while let Ok(msg) = msg_receiver.try_recv() {
                            let close = matches!(&msg, Message::Close(_));
                            if stream_sender.send(msg).await.is_err() || close {
                                break;
                            }
                        }
                        break;
                    }
                }
                result = msg_receiver.recv_async() => {
                    let Ok(msg) = result else {
                        break;
                    };
                    let close = matches!(&msg, Message::Close(_));
                    if let Err(e) = stream_sender.send(msg).await {
                        error!("Sending message to client failed: {e}, Client: {remote_address}");
                        break;
                    }
                    if close {
                        break;
                    }
                }
            }
        }

        // Tells the receive loop to stop. Once the client can no longer be
        // written to, a handler that keeps calling `send_message` would block
        // forever on a channel with no receiver.
        let _ = sender_done_tx.send(true);
    });

    // Handle on_open event
    // Call handler's on_open method for initialization
    if let Err(e) = handler.on_open(&session).await {
        error!("Event on_open processing failed: {e}, Client: {remote_address}, Path: {path}");

        let _ = handler.on_error(&session, e).await;
        let _ = handler.on_close(&session, None).await;
        let _ = shutdown_tx.send(true);
        let _ = sender_task.await;
        return;
    }

    info!("WS Connection established: {remote_address}, Path: {path}");

    // Receive client messages
    // Loop to receive and process messages from client
    // Call on_error method if processing fails
    //
    // The loop also ends when the sender task has gone, because from then on
    // nothing can reach the client and reading on would only postpone `on_close`.
    let mut close_frame = None;
    loop {
        tokio::select! {
            // Same reasoning as the shutdown above: an error means the sender
            // task is gone, which is exactly when this loop should stop.
            changed = sender_done_rx.changed() => {
                if changed.is_err() || *sender_done_rx.borrow() {
                    break;
                }
            }
            // A fresh timer each time round the loop, so it measures silence
            // since the last frame rather than the age of the connection.
            () = wait_for_idle(idle_timeout) => {
                info!(
                    "Closing a connection idle for {idle_timeout:?}: {remote_address}, Path: {path}"
                );
                session.close().await;
                break;
            }
            incoming = stream_receiver.next() => {
                let Some(result) = incoming else {
                    break;
                };
                match result {
                    Ok(msg) => {
                        if let Message::Close(frame) = &msg {
                            close_frame = frame.clone();
                        }
                        if let Err(e) = handler.on_message(&session, msg).await {
                            error!("Failed to process client message: {e}, Client: {remote_address}");
                            if let Err(e) = handler.on_error(&session, e).await {
                                error!("Failed to handle error event: {e}, Client: {remote_address}");
                            }
                            break;
                        }
                    }

                    Err(e) => {
                        error!("Failed to receive client message: {e}, Client: {remote_address}");
                        let boxed_err = Box::new(e);
                        let _ = handler.on_error(&session, boxed_err).await;
                        break;
                    }
                }
            }
        }
    }

    let _ = shutdown_tx.send(true);
    let _ = sender_task.await;

    // Handle connection closure
    // Call handler's on_close method for resource cleanup
    if let Err(e) = handler.on_close(&session, close_frame).await {
        error!("Failed to handle connection closure event: {e}, Client: {remote_address}");
    }
}

/// Waits out the idle limit, or forever when there is none.
///
/// Used as one arm of the receive loop's `select!`: the caller creates it again
/// on every pass, which is what makes the limit measure silence since the last
/// frame rather than the age of the connection. Without a limit the future never
/// completes, so that arm is simply never chosen.
async fn wait_for_idle(idle_timeout: Option<Duration>) {
    match idle_timeout {
        Some(timeout) => tokio::time::sleep(timeout).await,
        None => std::future::pending().await,
    }
}

/// End-to-end tests for the plain WebSocket path.
///
/// This path had no coverage at all before these: the SockJS transports have
/// their own connection loop, so nothing exercised `websocket_handle` or
/// `handle_socket`. Everything here goes through a real TCP connection and a real
/// handshake, because that is the only way the interceptor, upgrade and channel
/// sequencing is actually tested.
#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        error::Error,
        net::SocketAddr,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use axum::{
        Router,
        extract::ws::{CloseFrame, Message},
        routing::any,
    };
    use futures::{SinkExt, StreamExt};
    use next_web_core::{
        any::AnyValue, async_trait, error::BoxError,
        traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    };
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite;

    use super::websocket_handle;
    use crate::{
        autoconfigure::ws_properties::WebSocketProperties,
        config::{
            default_ws_handler_registry::DefaultWebSocketHandlerRegistry,
            ws_handler_registry::WebSocketHandlerRegistry,
        },
        server::{
            handshake_interceptor::HandshakeInterceptor,
            support::{ws_context::WebSocketContext, ws_session::WebSocketSession},
        },
        ws_handler::{WSResult, WebSocketHandler},
    };

    /// How long a test waits for something the server should do.
    const LIMIT: Duration = Duration::from_secs(3);

    /// What a handler saw, in order.
    #[derive(Default)]
    struct Events(Mutex<Vec<String>>);

    impl Events {
        fn record(&self, event: impl Into<String>) {
            self.0
                .lock()
                .expect("the events mutex is not poisoned")
                .push(event.into());
        }

        fn snapshot(&self) -> Vec<String> {
            self.0
                .lock()
                .expect("the events mutex is not poisoned")
                .clone()
        }
    }

    /// A handler that records what happened, and can be told to misbehave.
    struct TestHandler {
        events: Arc<Events>,
        /// How many messages to send from `on_open`.
        greeting: usize,
        /// Echo back this attribute instead of the message, when set.
        echo_attribute: Option<String>,
        /// Make `on_open` fail.
        fail_open: bool,
    }

    impl TestHandler {
        fn new(events: Arc<Events>) -> Self {
            Self {
                events,
                greeting: 0,
                echo_attribute: None,
                fail_open: false,
            }
        }
    }

    #[async_trait]
    impl WebSocketHandler for TestHandler {
        async fn on_open(&self, session: &WebSocketSession) -> WSResult<()> {
            self.events.record("open");
            if self.fail_open {
                return Err("on_open refused".into());
            }
            for index in 0..self.greeting {
                session
                    .send_message(Message::Text(format!("greeting {index}").into()))
                    .await
                    .map_err(|error| format!("greeting failed: {error:?}"))?;
            }
            Ok(())
        }

        async fn on_message(&self, session: &WebSocketSession, message: Message) -> WSResult<()> {
            let Message::Text(text) = message else {
                // Ping, Pong and Close arrive here too; only text is recorded so
                // that the assertions do not depend on the protocol's own frames.
                return Ok(());
            };

            self.events.record(format!("message:{text}"));
            let reply = match &self.echo_attribute {
                Some(name) => {
                    let value = session
                        .attribute(name)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "<missing>".to_owned());
                    format!("attribute {name}={value}")
                }
                None => format!("echo: {text}"),
            };

            session
                .send_message(Message::Text(reply.into()))
                .await
                .map_err(|error| format!("echo failed: {error:?}"))?;
            Ok(())
        }

        async fn on_error(
            &self,
            _session: &WebSocketSession,
            error: Box<dyn Error + Send + Sync>,
        ) -> WSResult<()> {
            self.events.record(format!("error:{error}"));
            Ok(())
        }

        async fn on_close(
            &self,
            _session: &WebSocketSession,
            close: Option<CloseFrame>,
        ) -> WSResult<()> {
            // The reason is recorded so a test can tell a close the client
            // explained from one it did not.
            match &close {
                Some(frame) => self.events.record(format!("close:{}", frame.reason)),
                None => self.events.record("close:no-frame"),
            }
            Ok(())
        }
    }

    /// Refuses every handshake without choosing a status.
    struct Refuser;

    #[async_trait]
    impl HandshakeInterceptor for Refuser {
        async fn before_handshake(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
            _attributes: &mut HashMap<String, AnyValue>,
        ) -> Result<bool, BoxError> {
            Ok(false)
        }
    }

    /// Fails, as an authentication service that is down would.
    struct Failing;

    #[async_trait]
    impl HandshakeInterceptor for Failing {
        async fn before_handshake(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
            _attributes: &mut HashMap<String, AnyValue>,
        ) -> Result<bool, BoxError> {
            Err("the authentication service is unreachable".into())
        }
    }

    /// Accepts the handshake, adding a header and an attribute on the way.
    struct Decorate;

    #[async_trait]
    impl HandshakeInterceptor for Decorate {
        async fn before_handshake(
            &self,
            _request: &mut dyn HttpRequest,
            response: &mut dyn HttpResponse,
            attributes: &mut HashMap<String, AnyValue>,
        ) -> Result<bool, BoxError> {
            response.insert_header("x-handshake", "set-by-interceptor");
            attributes.insert("principal".to_owned(), AnyValue::from("alice"));
            Ok(true)
        }
    }

    /// Serves one handler at `/ws` and returns the address it listens on.
    async fn serve(
        handler: Arc<dyn WebSocketHandler>,
        interceptors: Vec<Arc<dyn HandshakeInterceptor>>,
        properties: WebSocketProperties,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let mut registry = DefaultWebSocketHandlerRegistry::default();
        registry.add_handler(handler, vec!["/ws".to_owned()]);

        let context = WebSocketContext::new(properties, registry.handler_mapping(), interceptors);
        let app = Router::new()
            .route("/ws", any(websocket_handle))
            .with_state(Arc::new(context));

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let address = listener.local_addr().expect("the bound address");

        let server = tokio::spawn(async move {
            // The connection handler extracts `ConnectInfo`, so the server has to
            // be started with it. Serving the router with a bare `axum::serve`
            // would make every connection answer `500`.
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("the test server runs");
        });

        (address, server)
    }

    /// Reads frames until a text frame arrives and returns it.
    ///
    /// The protocol's own frames are skipped: the server opens with a ping, and
    /// the client answers with a pong that the server hands back to `on_message`.
    async fn next_text<S>(frames: &mut S) -> String
    where
        S: futures::Stream<Item = Result<tungstenite::Message, tungstenite::Error>> + Unpin,
    {
        loop {
            let frame = tokio::time::timeout(LIMIT, frames.next())
                .await
                .expect("a frame arrives in time")
                .expect("the stream is still open")
                .expect("the frame is not an error");

            if let tungstenite::Message::Text(text) = frame {
                return text.to_string();
            }
        }
    }

    /// The plain path probes the connection before the handler is reached, so the
    /// first frame a client sees is the server's ping.
    async fn expect_opening_ping<S>(frames: &mut S)
    where
        S: futures::Stream<Item = Result<tungstenite::Message, tungstenite::Error>> + Unpin,
    {
        let frame = tokio::time::timeout(LIMIT, frames.next())
            .await
            .expect("a frame arrives in time")
            .expect("the stream is still open")
            .expect("the frame is not an error");

        assert!(
            matches!(frame, tungstenite::Message::Ping(_)),
            "expected the opening ping, got {frame:?}"
        );
    }

    /// Waits until the recorded events satisfy `done`, so a test does not depend
    /// on a fixed sleep, and returns what was recorded either way.
    async fn wait_for(events: &Events, done: impl Fn(&[String]) -> bool) -> Vec<String> {
        for _ in 0..200 {
            let snapshot = events.snapshot();
            if done(&snapshot) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        events.snapshot()
    }

    /// The status of a refusal, read out of the handshake error.
    ///
    /// Compared as a number so the assertion does not depend on the `http` crate
    /// version tungstenite re-exports.
    fn http_status(error: &tungstenite::Error) -> u16 {
        match error {
            tungstenite::Error::Http(response) => response.status().as_u16(),
            other => panic!("expected an HTTP error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_client_reaches_the_handler_and_the_lifecycle_runs() {
        let events = Arc::new(Events::default());
        let (address, server) = serve(
            Arc::new(TestHandler::new(Arc::clone(&events))),
            Vec::new(),
            WebSocketProperties::default(),
        )
        .await;

        let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .expect("the handshake succeeds");

        expect_opening_ping(&mut client).await;

        client
            .send(tungstenite::Message::Text("hello".into()))
            .await
            .expect("the client sends");
        assert_eq!(next_text(&mut client).await, "echo: hello");

        // An explicit close frame, so the reason can be checked on the way back.
        client
            .send(tungstenite::Message::Close(Some(
                tungstenite::protocol::CloseFrame {
                    code: tungstenite::protocol::frame::coding::CloseCode::Normal,
                    reason: "bye".into(),
                },
            )))
            .await
            .expect("the client closes");

        let recorded = wait_for(&events, |events| {
            events.iter().any(|event| event.starts_with("close:"))
        })
        .await;

        assert!(recorded.contains(&"open".to_owned()), "{recorded:?}");
        assert!(
            recorded.contains(&"message:hello".to_owned()),
            "{recorded:?}"
        );
        // `on_close` is handed the frame the client sent, reason and all.
        assert!(recorded.contains(&"close:bye".to_owned()), "{recorded:?}");
        assert!(
            !recorded.iter().any(|event| event.starts_with("error:")),
            "{recorded:?}"
        );

        server.abort();
    }

    #[tokio::test]
    async fn a_handler_that_greets_from_on_open_is_not_blocked_by_a_full_channel() {
        let events = Arc::new(Events::default());
        let mut handler = TestHandler::new(Arc::clone(&events));
        handler.greeting = 4;

        // One slot in the outgoing channel, four greetings. When the sender task
        // was spawned after `on_open` instead of before it, nothing drained the
        // channel while `on_open` ran and the second send waited forever.
        let mut properties = WebSocketProperties::default();
        properties.set_msg_channel_capacity(1);

        let (address, server) = serve(Arc::new(handler), Vec::new(), properties).await;

        let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .expect("the handshake succeeds");
        expect_opening_ping(&mut client).await;

        for index in 0..4 {
            assert_eq!(next_text(&mut client).await, format!("greeting {index}"));
        }

        server.abort();
    }

    #[tokio::test]
    async fn a_refused_handshake_is_forbidden_and_never_reaches_the_handler() {
        let events = Arc::new(Events::default());
        let (address, server) = serve(
            Arc::new(TestHandler::new(Arc::clone(&events))),
            vec![Arc::new(Refuser)],
            WebSocketProperties::default(),
        )
        .await;

        let error = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .expect_err("the handshake is refused");

        // Refusing without choosing a status used to answer `200 OK`, which tells
        // the client its request succeeded when no connection was made.
        assert_eq!(http_status(&error), 403, "{error:?}");
        assert!(events.snapshot().is_empty(), "the handler must not run");

        server.abort();
    }

    #[tokio::test]
    async fn an_interceptor_that_fails_refuses_the_connection() {
        let events = Arc::new(Events::default());
        let (address, server) = serve(
            Arc::new(TestHandler::new(Arc::clone(&events))),
            vec![Arc::new(Failing)],
            WebSocketProperties::default(),
        )
        .await;

        let error = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .expect_err("a failing interceptor must fail closed");

        assert_eq!(http_status(&error), 403, "{error:?}");
        assert!(events.snapshot().is_empty(), "the handler must not run");

        server.abort();
    }

    #[tokio::test]
    async fn a_header_an_accepting_interceptor_added_reaches_the_handshake_response() {
        let events = Arc::new(Events::default());
        let (address, server) = serve(
            Arc::new(TestHandler::new(Arc::clone(&events))),
            vec![Arc::new(Decorate)],
            WebSocketProperties::default(),
        )
        .await;

        let (_client, response) = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .expect("the handshake succeeds");

        assert_eq!(
            response
                .headers()
                .get("x-handshake")
                .and_then(|value| value.to_str().ok()),
            Some("set-by-interceptor"),
            "the interceptor's header was dropped from the handshake response"
        );
        // The framing headers of the empty refusal must not have come along.
        assert!(
            response.headers().get("content-length").is_none(),
            "a content-length on a 101: {response:?}"
        );

        server.abort();
    }

    #[tokio::test]
    async fn handshake_attributes_reach_the_session() {
        let events = Arc::new(Events::default());
        let mut handler = TestHandler::new(Arc::clone(&events));
        handler.echo_attribute = Some("principal".to_owned());

        let (address, server) = serve(
            Arc::new(handler),
            vec![Arc::new(Decorate)],
            WebSocketProperties::default(),
        )
        .await;

        let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .expect("the handshake succeeds");
        expect_opening_ping(&mut client).await;

        client
            .send(tungstenite::Message::Text("who am i".into()))
            .await
            .expect("the client sends");

        // Before the session carried them, the interceptor's map was dropped when
        // the handshake ended and the handler could not see it.
        assert_eq!(next_text(&mut client).await, "attribute principal=alice");

        server.abort();
    }

    #[tokio::test]
    async fn a_client_that_goes_quiet_is_closed_after_the_configured_timeout() {
        let events = Arc::new(Events::default());
        let mut properties = WebSocketProperties::default();
        // The property counts seconds, so one is the smallest limit it can ask
        // for.
        properties.set_max_session_idle_timeout(1);

        let (address, server) = serve(
            Arc::new(TestHandler::new(Arc::clone(&events))),
            Vec::new(),
            properties,
        )
        .await;

        let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .expect("the handshake succeeds");
        expect_opening_ping(&mut client).await;

        // Say nothing from here on. The server should give up on its own rather
        // than hold the connection open indefinitely.
        let closed = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match client.next().await {
                    // The server's close frame, or the socket going away.
                    Some(Ok(tungstenite::Message::Close(_))) | None => break,
                    Some(Ok(_)) => continue,
                    Some(Err(_)) => break,
                }
            }
        })
        .await;

        assert!(
            closed.is_ok(),
            "an idle connection was never closed although the limit was one second"
        );

        let recorded = wait_for(&events, |events| {
            events.iter().any(|event| event.starts_with("close:"))
        })
        .await;
        assert!(
            recorded.iter().any(|event| event.starts_with("close:")),
            "the handler was not told the connection ended: {recorded:?}"
        );

        server.abort();
    }
}
