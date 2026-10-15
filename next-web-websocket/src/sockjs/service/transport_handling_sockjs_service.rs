use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use axum::{
    body::Body,
    extract::{Request, ws::WebSocketUpgrade},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::Response,
};
use dashmap::DashMap;
use next_web_core::{any::AnyValue, error::BoxError};
use tracing::{debug, error, warn};

use crate::{
    server::handshake_interceptor::HandshakeInterceptor,
    server::support::origin_handshake_interceptor::OriginHandshakeInterceptor,
    sockjs::{
        codec::{JsonSockJsMessageCodec, SockJsMessageCodec},
        info::SockJsInfo,
        session::SockJsSession,
        transport::{SockJsServiceConfig, TransportHandler, TransportType},
    },
    ws_handler::WebSocketHandler,
};

use super::{Authorization, SockJsService};

/// Base implementation of SockJS service with transport handling.
///
/// This service manages:
/// - Session lifecycle (create, retrieve, expire)
/// - Transport handler registration and dispatch
/// - Configuration and interceptors
pub struct TransportHandlingSockJsService {
    /// Service configuration
    config: Arc<SockJsServiceConfig>,

    /// Active sessions (session_id -> SockJsSession)
    sessions: Arc<DashMap<String, Arc<SockJsSession>>>,

    /// Ensures that the service owns at most one periodic cleanup task.
    cleanup_task_started: AtomicBool,

    /// Transport handlers (transport_type -> handler)
    transport_handlers: HashMap<TransportType, Arc<dyn TransportHandler>>,

    /// Handshake interceptors
    interceptors: Vec<Arc<dyn HandshakeInterceptor>>,

    /// Message codec
    codec: Arc<dyn SockJsMessageCodec>,

    /// Application handler used by sessions created for this endpoint.
    websocket_handler: Option<Arc<dyn WebSocketHandler>>,
}

impl TransportHandlingSockJsService {
    /// Creates a new transport handling SockJS service.
    ///
    /// # Arguments
    ///
    /// * `config` - Service configuration
    /// * `transport_handlers` - Map of transport handlers
    pub fn new(
        config: SockJsServiceConfig,
        transport_handlers: HashMap<TransportType, Arc<dyn TransportHandler>>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            sessions: Arc::new(DashMap::new()),
            cleanup_task_started: AtomicBool::new(false),
            transport_handlers,
            interceptors: Vec::new(),
            codec: Arc::new(JsonSockJsMessageCodec),
            websocket_handler: None,
        }
    }

    /// Binds the endpoint's application WebSocket handler to its sessions.
    pub fn set_websocket_handler(&mut self, handler: Arc<dyn WebSocketHandler>) {
        self.websocket_handler = Some(handler);
    }

    /// Sets the message codec.
    pub fn set_codec(&mut self, codec: Arc<dyn SockJsMessageCodec>) {
        self.codec = codec;
    }

    /// Adds a handshake interceptor.
    pub fn add_interceptor(&mut self, interceptor: Arc<dyn HandshakeInterceptor>) {
        self.interceptors.push(interceptor);
    }

    /// Sets all handshake interceptors.
    pub fn set_interceptors(&mut self, interceptors: Vec<Arc<dyn HandshakeInterceptor>>) {
        self.interceptors = interceptors;
    }

    /// Handles the /info endpoint request.
    ///
    /// Returns JSON with server capabilities and configuration.
    fn handle_info(&self) -> Response {
        SockJsInfo::with_origins(
            self.config.websocket_enabled,
            self.config.session_cookie_needed,
            // Report the origins this service actually accepts. The field used to
            // be hard-coded to `*:*`, so a deployment with a strict allowlist
            // still told every client that any origin was welcome.
            self.advertised_origins(),
        )
        .to_response()
    }

    /// The origins to advertise in `/info`.
    ///
    /// A service with no origin restriction accepts anything, which SockJS writes
    /// as `*:*`. With one configured, the lists are reported as they are.
    fn advertised_origins(&self) -> Vec<String> {
        if self.config.allowed_origins.is_empty() && self.config.allowed_origin_patterns.is_empty() {
            return vec!["*:*".to_string()];
        }

        self.config
            .allowed_origins
            .iter()
            .chain(self.config.allowed_origin_patterns.iter())
            .cloned()
            .collect()
    }

    /// Reports whether this request may use `session`, fixing the session to its
    /// first caller on the way.
    ///
    /// Session ids come from the client, so without a check here a request that
    /// guesses or learns another client's id could read its messages and inject
    /// into its handler. Checked even for a session this request just created —
    /// where it trivially passes — so that there is one path to reason about.
    ///
    /// Two claims are available. An **identity**, when the service names an
    /// attribute for it and the session carries a comparable value: that decides,
    /// because it is the stronger claim and the only one that still means
    /// anything behind a reverse proxy. Otherwise the **address** the session came
    /// from, which costs nothing and helps when the deployment really does see
    /// client addresses.
    fn accepts_session(
        &self,
        session: &SockJsSession,
        session_id: &str,
        remote_addr: SocketAddr,
        presented: &HashMap<String, AnyValue>,
        created: bool,
    ) -> bool {
        /// Renders an identity to something two requests can be compared by.
        ///
        /// `AnyValue` cannot be compared — it documents that — and its
        /// `BoxedValue` variant prints as a placeholder, so comparing rendered
        /// values would make two different principals look equal. Only the
        /// variants with an unambiguous textual form are accepted, and a session
        /// whose identity is one of the others is refused rather than let through.
        fn comparable(value: &AnyValue) -> Option<String> {
            match value {
                AnyValue::String(_) | AnyValue::Number(_) | AnyValue::Boolean(_) => {
                    Some(value.to_string())
                }
                _ => None,
            }
        }

        // The request that created the session is by definition its owner, and
        // there is nothing yet to compare it against — including when the identity
        // it supplied is one this check cannot read, which would otherwise refuse
        // the very request that made the session. Its address is recorded either
        // way, so the pin still applies if the identity does not.
        if created {
            session.bind_to(remote_addr.ip());
            return true;
        }

        if let Some(name) = self.config.session_identity_attribute.as_deref()
            && let Some(stored) = session.attributes().get(name)
        {
            let stored = comparable(stored);
            let presented = presented.get(name).and_then(comparable);
            if stored.is_some() && stored == presented {
                // The identity decides; the address is not consulted, because it
                // would add rejections for a client that legitimately changed
                // address without adding any strength.
                return true;
            }

            // Either a different identity, or one the check cannot read. Refusing
            // is the honest answer: letting it through would make the control
            // theatre, which is exactly what the address pin becomes behind a
            // proxy.
            warn!(
                session = %session_id,
                %remote_addr,
                attribute = name,
                readable = stored.is_some(),
                "Refusing a SockJS request whose identity does not match its session"
            );
            return false;
        }

        session.bind_to(remote_addr.ip());

        if !self.config.session_pinning || session.belongs_to(remote_addr.ip()) {
            return true;
        }

        // 404 rather than 403: a SockJS client treats a missing session as "start a
        // new one", which is the right recovery, and the answer does not confirm
        // that the id exists.
        warn!(
            session = %session_id,
            %remote_addr,
            bound = ?session.bound_ip(),
            "Refusing a SockJS request from an address the session does not belong to"
        );
        false
    }

    async fn authorize_request(
        &self,
        request: &mut Request,
        run_custom_interceptors: bool,
    ) -> Result<Authorization, BoxError> {
        let origin_interceptor = OriginHandshakeInterceptor::new(
            self.config.allowed_origins.clone(),
            self.config.allowed_origin_patterns.clone(),
        );
        let mut response = Response::default();
        let mut attributes = HashMap::new();

        // A refusal carries the response the interceptor built; a failure is
        // treated as a refusal, because letting a broken check through would be
        // the wrong default for a handshake. Setting the status here means an
        // interceptor that failed before choosing one still produces a
        // meaningful answer rather than `200 OK`.
        fn refuse_on_failure(response: &mut Response, interceptor: &str, failure: BoxError) {
            error!(
                interceptor = %interceptor,
                "SockJS handshake interceptor failed, refusing the connection: {failure}"
            );
            *response.status_mut() = StatusCode::FORBIDDEN;
        }

        match origin_interceptor
            .before_handshake(request, &mut response, &mut attributes)
            .await
        {
            Ok(true) => {}
            Ok(false) => return Ok(Authorization::refused(response)),
            Err(failure) => {
                refuse_on_failure(&mut response, "origin", failure);
                return Ok(Authorization::refused(response));
            }
        }

        if run_custom_interceptors {
            for interceptor in &self.interceptors {
                match interceptor
                    .before_handshake(request, &mut response, &mut attributes)
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => return Ok(Authorization::refused(response)),
                    Err(failure) => {
                        refuse_on_failure(&mut response, "custom", failure);
                        return Ok(Authorization::refused(response));
                    }
                }
            }
        }

        // What the interceptors collected travels with the request so that the
        // session it creates can hand it to the handler, which is what the
        // `attributes` parameter of `before_handshake` promises.
        Ok(Authorization::accepted(attributes))
    }

    fn add_cors_headers_for_origin(&self, origin: Option<&HeaderValue>, response: &mut Response) {
        if self.config.suppress_cors {
            return;
        }

        let Some(origin) = origin.cloned() else {
            return;
        };

        response
            .headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
            HeaderValue::from_static("true"),
        );

        let vary = response
            .headers_mut()
            .entry(header::VARY)
            .or_insert_with(|| HeaderValue::from_static("Origin"));
        if !vary
            .to_str()
            .unwrap_or_default()
            .split(',')
            .any(|value| value.trim().eq_ignore_ascii_case("origin"))
            && let Ok(value) =
                HeaderValue::from_str(&format!("{}, Origin", vary.to_str().unwrap_or_default()))
        {
            *vary = value;
        }
    }

    async fn handle_preflight(&self, request: &mut Request) -> Result<Response, BoxError> {
        if let Some(response) = self.authorize_request(request, false).await?.refusal {
            return Ok(response);
        }

        let path = request.uri().path();
        let endpoint = path.rsplit('/').next().unwrap_or_default();
        let (allowed_methods, allow_methods_header) = match endpoint {
            "info" => (&["GET"][..], "GET, OPTIONS"),
            "xhr_streaming" => (&["GET", "POST"][..], "GET, POST, OPTIONS"),
            "websocket" => (&["GET"][..], "GET, OPTIONS"),
            "xhr_send" => (&["POST"][..], "POST, OPTIONS"),
            "xhr" => (&["GET", "POST"][..], "GET, POST, OPTIONS"),
            _ => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())?);
            }
        };
        let requested_method = request
            .headers()
            .get(header::ACCESS_CONTROL_REQUEST_METHOD)
            .and_then(|value| value.to_str().ok());
        if requested_method.is_some_and(|method| !allowed_methods.contains(&method)) {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::empty())?);
        }

        let mut response = Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())?;
        self.add_cors_headers_for_origin(request.headers().get(header::ORIGIN), &mut response);
        if !self.config.suppress_cors {
            response.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static(allow_methods_header),
            );
            response.headers_mut().insert(
                header::VARY,
                HeaderValue::from_static(
                    "Origin, Access-Control-Request-Method, Access-Control-Request-Headers",
                ),
            );
            if let Some(requested_headers) = request
                .headers()
                .get(header::ACCESS_CONTROL_REQUEST_HEADERS)
                .cloned()
            {
                response
                    .headers_mut()
                    .insert(header::ACCESS_CONTROL_ALLOW_HEADERS, requested_headers);
            }
            response.headers_mut().insert(
                header::ACCESS_CONTROL_MAX_AGE,
                HeaderValue::from_static("600"),
            );
        }
        Ok(response)
    }

    /// Extracts session ID and transport type from request path.
    ///
    /// Expected path format: /{endpoint}/{server-id}/{session-id}/{transport}
    fn parse_path(path: &str) -> Option<(String, TransportType)> {
        let parts: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() < 4 {
            return None;
        }

        let session_id = parts[parts.len() - 2].to_string();
        let transport = match parts[parts.len() - 1] {
            "websocket" => TransportType::WebSocket,
            "xhr_streaming" => TransportType::XhrStreaming,
            "xhr" | "xhr_send" => TransportType::XhrPolling,
            "eventsource" => TransportType::EventSource,
            _ => return None,
        };

        Some((session_id, transport))
    }

    /// Starts a weakly-owned periodic cleanup task after the service first
    /// handles a request. The task exits automatically when the service drops.
    fn start_cleanup_task(&self) {
        if self
            .cleanup_task_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let sessions = Arc::downgrade(&self.sessions);
        // The sweep can only be as precise as how often it runs, so expiry uses
        // the same clamped value as the interval. Comparing against the raw
        // `disconnect_delay` let a small setting expire after a millisecond while
        // being checked once a second, which reads as a session lifetime of a
        // second and is not what the number says.
        let disconnect_delay = self.config.disconnect_delay.clamp(1_000, 30_000);
        let interval = Duration::from_millis(disconnect_delay);
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            self.cleanup_task_started.store(false, Ordering::Release);
            return;
        };

        runtime.spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let Some(sessions) = sessions.upgrade() else {
                    break;
                };
                Self::cleanup_sessions(&sessions, disconnect_delay).await;
            }
        });
    }

    async fn cleanup_sessions(
        sessions: &DashMap<String, Arc<SockJsSession>>,
        disconnect_delay: u64,
    ) {
        let mut expired = Vec::new();

        for entry in sessions.iter() {
            if entry.value().is_expired(disconnect_delay).await {
                expired.push(entry.key().clone());
            }
        }

        for session_id in expired {
            debug!("Removing expired session: {}", session_id);
            if let Some((_, session)) = sessions.remove(&session_id) {
                // Expiration is the close event for abandoned HTTP transports.
                // Notify the application and stop its output pump before
                // releasing the service's session reference.
                session.notify_close(None).await;
            }
        }
    }
}

#[next_web_core::async_trait]
impl SockJsService for TransportHandlingSockJsService {
    async fn handle_request(&self, mut req: Request) -> Result<Response, BoxError> {
        self.start_cleanup_task();
        let path = req.uri().path().to_owned();

        if req.method() == Method::OPTIONS {
            return self.handle_preflight(&mut req).await;
        }

        // Handle /info endpoint
        if path.ends_with("/info") {
            if req.method() != Method::GET {
                return Ok(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .header(header::ALLOW, "GET, OPTIONS")
                    .body(Body::empty())?);
            }
            if let Some(response) = self.authorize_request(&mut req, false).await?.refusal {
                return Ok(response);
            }
            let mut response = self.handle_info();
            self.add_cors_headers_for_origin(req.headers().get(header::ORIGIN), &mut response);
            return Ok(response);
        }

        // Parse session ID and transport from path
        let Some((session_id, transport_type)) = Self::parse_path(&path) else {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?);
        };

        debug!(
            "SockJS request: session={}, transport={:?}",
            session_id, transport_type
        );

        let authorization = self.authorize_request(&mut req, true).await?;
        if let Some(response) = authorization.refusal {
            return Ok(response);
        }

        let endpoint = path.rsplit('/').next().unwrap_or_default();
        let method_is_valid = match endpoint {
            // SockJS clients establish XHR receive transports with POST and
            // an empty body. Keep GET receive requests working for callers
            // that used the original Next Web endpoint behavior.
            "xhr" | "xhr_streaming" => req.method() == Method::GET || req.method() == Method::POST,
            "xhr_send" => req.method() == Method::POST,
            _ => req.method() == Method::GET || req.method() == Method::POST,
        };
        if !method_is_valid {
            let allow = match endpoint {
                "xhr" | "xhr_streaming" => "GET, POST, OPTIONS",
                "xhr_send" => "POST, OPTIONS",
                _ => "GET, POST, OPTIONS",
            };
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::ALLOW, allow)
                .body(Body::empty())?);
        }

        if req.method() != Method::GET && req.method() != Method::POST {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::ALLOW, "GET, POST, OPTIONS")
                .body(Body::empty())?);
        }

        let origin = req.headers().get(header::ORIGIN).cloned();

        // The transport is looked up before anything is created. An unregistered
        // one — `eventsource` is parsed out of the URL but no handler is ever
        // registered for it — would otherwise create a session and then answer
        // 404, leaving that session behind for the sweep to collect.
        let Some(handler) = self.transport_handlers.get(&transport_type) else {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?);
        };

        // Get or create session
        let remote_addr = req
            .extensions()
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0)
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap());

        // Kept for the identity check below, which needs what this request
        // presents alongside what the session was created with.
        let presented = authorization.attributes.clone();

        let mut created = false;
        let session = self
            .sessions
            .entry(session_id.clone())
            .or_insert_with(|| {
                created = true;
                let session = Arc::new(SockJsSession::with_attributes(
                    session_id.clone(),
                    self.codec.clone(),
                    remote_addr,
                    req.headers().clone(),
                    // The request that creates the session decides its
                    // attributes; later polls of the same session do not, since
                    // the session already exists by then.
                    authorization.attributes,
                ));
                session.set_queue_capacity(self.config.http_message_cache_size);
                session
            })
            .clone();
        if let Some(app_handler) = &self.websocket_handler {
            session.set_websocket_handler(app_handler.clone());
        }

        if !self.accepts_session(&session, &session_id, remote_addr, &presented, created) {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?);
        }

        // The transport was resolved above, before the session was created.
        debug_assert!(self.transport_handlers.contains_key(&transport_type));

        // Delegate to transport handler
        let response = handler
            .handle_request(req, session, self.config.clone())
            .await?;

        let mut response = response;
        self.add_cors_headers_for_origin(origin.as_ref(), &mut response);

        Ok(response)
    }

    async fn authorize_websocket_request(
        &self,
        request: &mut Request,
    ) -> Result<Authorization, BoxError> {
        self.authorize_request(request, true).await
    }

    fn add_cors_headers(&self, request: &Request, response: &mut Response) {
        self.add_cors_headers_for_origin(request.headers().get(header::ORIGIN), response);
    }

    async fn handle_websocket_upgrade(
        &self,
        ws: WebSocketUpgrade,
        session_id: String,
        headers: HeaderMap,
        remote_addr: SocketAddr,
        attributes: HashMap<String, AnyValue>,
    ) -> Result<Response, BoxError> {
        self.start_cleanup_task();
        if !self.config.websocket_enabled {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?);
        }

        // Kept for the identity check below, which needs what this request
        // presents alongside what the session was created with.
        let presented = attributes.clone();

        let mut created = false;
        let session = self
            .sessions
            .entry(session_id.clone())
            .or_insert_with(|| {
                created = true;
                let session = Arc::new(SockJsSession::with_attributes(
                    session_id.clone(),
                    self.codec.clone(),
                    remote_addr,
                    headers,
                    attributes,
                ));
                session.set_queue_capacity(self.config.http_message_cache_size);
                session
            })
            .clone();
        if let Some(app_handler) = &self.websocket_handler {
            session.set_websocket_handler(app_handler.clone());
        }

        if !self.accepts_session(&session, &session_id, remote_addr, &presented, created) {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?);
        }

        let transport_handler = self
            .transport_handlers
            .get(&TransportType::WebSocket)
            .ok_or_else(|| "WebSocket transport is disabled".to_string())?;
        transport_handler
            .handle_websocket_upgrade(ws, session, self.config.clone())
            .await
    }

    fn get_session(&self, session_id: &str) -> Option<Arc<SockJsSession>> {
        self.sessions.get(session_id).map(|e| e.clone())
    }

    async fn create_session(
        &self,
        session_id: String,
        remote_addr: SocketAddr,
        headers: axum::http::HeaderMap,
    ) -> Arc<SockJsSession> {
        self.start_cleanup_task();
        let session = Arc::new(SockJsSession::new(
            session_id.clone(),
            self.codec.clone(),
            remote_addr,
            headers,
        ));
        if let Some(handler) = &self.websocket_handler {
            session.set_websocket_handler(handler.clone());
        }

        self.sessions.insert(session_id, session.clone());
        session
    }

    fn config(&self) -> Arc<SockJsServiceConfig> {
        self.config.clone()
    }

    fn interceptors(&self) -> &[Arc<dyn HandshakeInterceptor>] {
        &self.interceptors
    }
}
