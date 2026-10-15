use std::{collections::HashMap, sync::Arc};

use crate::{
    server::handshake_interceptor::HandshakeInterceptor,
    sockjs::{
        codec::SockJsMessageCodec,
        service::{DefaultSockJsService, SockJsService, TransportHandlingSockJsService},
        transport::{SockJsServiceConfig, TransportHandler},
    },
    ws_handler::WebSocketHandler,
};

/// Configuration builder for SockJS fallback options.
///
/// SockJS clients can fall back to HTTP transports when native WebSocket is
/// unavailable or an upgrade is blocked. The built-in fallback transports are
/// XHR streaming and XHR polling; legacy iframe and XDomainRequest transports
/// are not included.
///
/// # Fallback Mechanism
/// 1. **WebSocket** - Native connection (preferred)
/// 2. **HTTP Streaming** - Long-lived HTTP with chunked transfer
/// 3. **HTTP Long Polling** - Traditional request/response cycle
///
/// # Usage Example
/// ```rust,ignore
/// registration
///     .with_sock_js()
///     .set_heartbeat_time(25000)
///     .set_disconnect_delay(5000)
///     .set_websocket_enabled(true);
/// ```
#[derive(Default)]
pub struct SockJsServiceRegistration {
    /// Optional scheduler handle retained for Spring-style configuration
    /// compatibility. Built-in heartbeats currently use the Tokio runtime.
    task_scheduler: Option<Arc<dyn Send + Sync>>,

    /// URL of SockJS client library for iframe-based transports.
    ///
    /// Transports like "htmlfile" need to load SockJS from a trusted domain.
    /// Default: "https://cdn.jsdelivr.net/sockjs/1.0.0/sockjs.min.js"
    ///
    /// **Has no effect**: the iframe-based transports are not implemented, so
    /// nothing serves this URL or reads this value. It is kept so that the
    /// setting keeps its shape if they are added.
    client_library_url: Option<String>,

    /// Maximum bytes sent over a single HTTP streaming request.
    ///
    /// Streaming transports must recycle connections to free client memory.
    /// Setting this to 1 effectively disables streaming (forces polling behavior).
    /// Default: 128KB (128 * 1024)
    stream_bytes_limit: Option<usize>,

    /// Whether JSESSIONID cookie is required for the application.
    ///
    /// Reported in `/info` so clients can account for cookie-based session
    /// affinity. This does not enable legacy iframe transports.
    /// Default: true
    session_cookie_needed: Option<bool>,

    /// Heartbeat interval in milliseconds.
    ///
    /// Server sends 'h' frame to keep connection alive when no messages sent.
    /// Prevents proxies from closing idle connections.
    /// Default: 25,000 (25 seconds)
    heartbeat_time: Option<u64>,

    /// Disconnect delay in milliseconds.
    ///
    /// Time before considering a client disconnected after losing receiving connection.
    /// Default: 5,000 (5 seconds)
    disconnect_delay: Option<u64>,

    /// Number of server-to-client messages cached while waiting for next poll.
    ///
    /// All transports use this since streaming also recycles periodically.
    /// Default: 100
    http_message_cache_size: Option<usize>,

    /// Enable/disable WebSocket transport.
    ///
    /// Useful when load balancers don't support WebSocket.
    /// Default: true
    websocket_enabled: Option<bool>,

    /// Custom transport handlers (replaces defaults)
    transport_handlers: Vec<Arc<dyn TransportHandler>>,

    /// Transport handler overrides (modifies specific defaults)
    transport_handler_overrides: Vec<Arc<dyn TransportHandler>>,

    /// Handshake interceptors for authentication/validation
    interceptors: Vec<Arc<dyn HandshakeInterceptor>>,

    /// Allowed origins for CORS
    allowed_origins: Vec<String>,

    /// Allowed origin patterns for CORS (supports wildcards)
    allowed_origin_patterns: Vec<String>,

    /// Whether a session may only be used from the address that created it.
    /// Default: true.
    session_pinning: Option<bool>,

    /// Attribute that identifies who a session belongs to. Default: unset, so the
    /// identity is not checked.
    session_identity_attribute: Option<String>,

    /// Suppress automatic CORS header addition
    /// Default: false
    suppress_cors: Option<bool>,

    /// Message codec for encoding/decoding SockJS frames
    message_codec: Option<Arc<dyn SockJsMessageCodec>>,
}

impl SockJsServiceRegistration {
    /// Set the task scheduler for heartbeat management.
    ///
    /// **Has no effect**: the value is stored and can be read back, but nothing
    /// uses it, because heartbeat tasks are spawned on the active Tokio runtime.
    /// It is kept for configuration compatibility.
    pub fn set_task_scheduler(&mut self, scheduler: Arc<dyn Send + Sync>) -> &mut Self {
        self.task_scheduler = Some(scheduler);
        self
    }

    /// Set the SockJS client library URL for iframe transports.
    ///
    /// # Arguments
    /// * `url` - Absolute URL or relative path (e.g., "../../sockjs.min.js")
    ///
    /// # Note
    /// Relative URLs must account for the SockJS endpoint depth.
    /// For endpoint "/sockjs", iframe is "/sockjs/iframe.html",
    /// so "../../" navigates above the mapping.
    pub fn set_client_library_url(&mut self, url: String) -> &mut Self {
        self.client_library_url = Some(url);
        self
    }

    /// Set the stream bytes limit for HTTP streaming transports.
    ///
    /// # Arguments
    /// * `limit` - Bytes threshold (set to 1 to disable streaming)
    pub fn set_stream_bytes_limit(&mut self, limit: usize) -> &mut Self {
        self.stream_bytes_limit = Some(limit);
        self
    }

    /// Set whether session cookie (JSESSIONID) is needed.
    ///
    /// # Arguments
    /// * `needed` - true for session-based apps, false if stateless
    ///
    /// This value is reported by the `/info` endpoint. It does not enable
    /// legacy transports that depend on cookies.
    pub fn set_session_cookie_needed(&mut self, needed: bool) -> &mut Self {
        self.session_cookie_needed = Some(needed);
        self
    }

    /// Set the heartbeat interval in milliseconds.
    ///
    /// # Arguments
    /// * `time` - Milliseconds between heartbeat frames
    pub fn set_heartbeat_time(&mut self, time: u64) -> &mut Self {
        self.heartbeat_time = Some(time);
        self
    }

    /// Set the disconnect delay in milliseconds.
    ///
    /// # Arguments
    /// * `delay` - Milliseconds to wait before considering client disconnected
    pub fn set_disconnect_delay(&mut self, delay: u64) -> &mut Self {
        self.disconnect_delay = Some(delay);
        self
    }

    /// Set the HTTP message cache size.
    ///
    /// # Arguments
    /// * `size` - Number of messages to cache per session
    pub fn set_http_message_cache_size(&mut self, size: usize) -> &mut Self {
        self.http_message_cache_size = Some(size);
        self
    }

    /// Enable or disable WebSocket transport.
    ///
    /// # Arguments
    /// * `enabled` - false to disable WebSocket (use only HTTP fallbacks)
    pub fn set_websocket_enabled(&mut self, enabled: bool) -> &mut Self {
        self.websocket_enabled = Some(enabled);
        self
    }

    /// Set custom transport handlers (replaces all defaults).
    ///
    /// # Arguments
    /// * `handlers` - Complete list of transport handlers
    ///
    /// # Note
    /// Mutually exclusive with `set_transport_handler_overrides`.
    pub fn set_transport_handlers(
        &mut self,
        handlers: Vec<Arc<dyn TransportHandler>>,
    ) -> &mut Self {
        self.transport_handlers = handlers;
        self
    }

    /// Set transport handler overrides (modifies specific defaults).
    ///
    /// # Arguments
    /// * `overrides` - Handlers to override by transport type
    ///
    /// # Note
    /// Mutually exclusive with `set_transport_handlers`.
    pub fn set_transport_handler_overrides(
        &mut self,
        overrides: Vec<Arc<dyn TransportHandler>>,
    ) -> &mut Self {
        self.transport_handler_overrides = overrides;
        self
    }

    /// Set handshake interceptors.
    ///
    /// # Arguments
    /// * `interceptors` - List of interceptors for authentication/validation
    pub fn set_interceptors(
        &mut self,
        interceptors: Vec<Arc<dyn HandshakeInterceptor>>,
    ) -> &mut Self {
        self.interceptors = interceptors;
        self
    }

    /// Set allowed origins for CORS.
    ///
    /// # Arguments
    /// * `origins` - List of allowed origin URLs
    pub fn set_allowed_origins(&mut self, origins: Vec<String>) -> &mut Self {
        self.allowed_origins = origins;
        self
    }

    /// Set the attribute that identifies who a SockJS session belongs to.
    ///
    /// A handshake interceptor puts the identity in that attribute — use
    /// [`crate::SESSION_IDENTITY_ATTRIBUTE`] or a name of your own — and from then
    /// on only a request presenting the same value may use that session. This is
    /// the check that still works behind a reverse proxy, where every client
    /// shares the proxy's address and the IP pin cannot tell them apart.
    ///
    /// Off unless this is called. A session whose creating request set no such
    /// attribute falls back to the IP pin rather than being refused, so an
    /// anonymous session keeps working.
    pub fn set_session_identity_attribute(&mut self, name: impl Into<String>) -> &mut Self {
        self.session_identity_attribute = Some(name.into());
        self
    }

    /// Set whether a session may only be used from the address that created it.
    ///
    /// On by default. The session id comes from the client, so with this off a
    /// request that guesses or learns another client's id can read its messages
    /// and inject into its handler. Turn it off only when clients legitimately
    /// change address mid-session and the ids cannot be guessed.
    pub fn set_session_pinning(&mut self, pinning: bool) -> &mut Self {
        self.session_pinning = Some(pinning);
        self
    }

    /// Set allowed origin patterns for CORS.
    ///
    /// # Arguments
    /// * `patterns` - List of origin patterns (supports wildcards)
    pub fn set_allowed_origin_patterns(&mut self, patterns: Vec<String>) -> &mut Self {
        self.allowed_origin_patterns = patterns;
        self
    }

    /// Suppress automatic CORS header addition.
    ///
    /// # Arguments
    /// * `suppress` - true to disable automatic CORS headers
    pub fn set_suppress_cors(&mut self, suppress: bool) -> &mut Self {
        self.suppress_cors = Some(suppress);
        self
    }

    /// Set the message codec for encoding/decoding.
    ///
    /// # Arguments
    /// * `codec` - Custom codec implementation
    ///
    /// # Default
    /// Uses JSON-based codec (similar to JsonSockJsMessageCodec)
    pub fn set_message_codec(&mut self, codec: Arc<dyn SockJsMessageCodec>) -> &mut Self {
        self.message_codec = Some(codec);
        self
    }

    /// Build and return the configured SockJS service.
    ///
    /// # Returns
    /// A configured `TransportHandlingSockJsService` instance
    ///
    /// # Panics
    /// - If both `transport_handlers` and `transport_handler_overrides` are set
    pub fn get_sockjs_service(&self) -> Box<dyn SockJsService> {
        self.build_sockjs_service(None, &[])
    }

    /// Build a SockJS service connected to the application WebSocket handler.
    pub(crate) fn get_sockjs_service_with_handler(
        &self,
        handler: Arc<dyn WebSocketHandler>,
        additional_interceptors: &[Arc<dyn HandshakeInterceptor>],
    ) -> Box<dyn SockJsService> {
        self.build_sockjs_service(Some(handler), additional_interceptors)
    }

    /// Builds a SockJS service bound to an application WebSocket handler.
    ///
    /// Use this when mounting a SockJS service directly onto an Axum router.
    /// Framework applications should usually enable the fallback on a
    /// registered endpoint with
    /// [`WebSocketHandlerRegistration::with_sock_js`](crate::config::WebSocketHandlerRegistration::with_sock_js).
    pub fn get_sockjs_service_with_application_handler(
        &self,
        handler: Arc<dyn WebSocketHandler>,
    ) -> Box<dyn SockJsService> {
        self.build_sockjs_service(Some(handler), &[])
    }

    fn build_sockjs_service(
        &self,
        handler: Option<Arc<dyn WebSocketHandler>>,
        additional_interceptors: &[Arc<dyn HandshakeInterceptor>],
    ) -> Box<dyn SockJsService> {
        // Validate configuration
        if !self.transport_handlers.is_empty() && !self.transport_handler_overrides.is_empty() {
            panic!("Cannot specify both transport_handlers and transport_handler_overrides");
        }

        // Build configuration
        let mut config = SockJsServiceConfig::default();

        if let Some(ref url) = self.client_library_url {
            config.client_library_url = url.clone();
        }
        if let Some(limit) = self.stream_bytes_limit {
            config.stream_bytes_limit = limit;
        }
        if let Some(needed) = self.session_cookie_needed {
            config.session_cookie_needed = needed;
        }
        if let Some(time) = self.heartbeat_time {
            config.heartbeat_time = time;
        }
        if let Some(delay) = self.disconnect_delay {
            config.disconnect_delay = delay;
        }
        if let Some(size) = self.http_message_cache_size {
            config.http_message_cache_size = size;
        }
        if let Some(enabled) = self.websocket_enabled {
            config.websocket_enabled = enabled;
        }
        if let Some(suppress) = self.suppress_cors {
            config.suppress_cors = suppress;
        }

        config.allowed_origins = self.allowed_origins.clone();
        config.allowed_origin_patterns = self.allowed_origin_patterns.clone();
        if let Some(pinning) = self.session_pinning {
            config.session_pinning = pinning;
        }
        if let Some(name) = self.session_identity_attribute.clone() {
            config.session_identity_attribute = Some(name);
        }

        // Create service with default or custom handlers
        let mut service = if !self.transport_handlers.is_empty() {
            // Use custom handlers
            let mut handlers = HashMap::new();
            for handler in &self.transport_handlers {
                handlers.insert(handler.transport_type(), handler.clone());
            }
            TransportHandlingSockJsService::new(config, handlers)
        } else if !self.transport_handler_overrides.is_empty() {
            // Use default service with overrides
            let mut handlers = HashMap::new();
            for handler in &self.transport_handler_overrides {
                handlers.insert(handler.transport_type(), handler.clone());
            }
            DefaultSockJsService::with_overrides(config, handlers)
        } else {
            // Use all defaults
            DefaultSockJsService::new(config)
        };

        // Set interceptors
        if !self.interceptors.is_empty() || !additional_interceptors.is_empty() {
            let mut interceptors = self.interceptors.clone();
            for interceptor in additional_interceptors {
                if !interceptors
                    .iter()
                    .any(|registered| Arc::ptr_eq(registered, interceptor))
                {
                    interceptors.push(interceptor.clone());
                }
            }
            service.set_interceptors(interceptors);
        }

        // Set message codec
        if let Some(ref codec) = self.message_codec {
            service.set_codec(codec.clone());
        }
        if let Some(handler) = handler {
            service.set_websocket_handler(handler);
        }

        Box::new(service)
    }

    /// Get the configured task scheduler reference.
    pub fn get_task_scheduler(&self) -> Option<&Arc<dyn Send + Sync>> {
        self.task_scheduler.as_ref()
    }
}

#[cfg(test)]
#[path = "sock_js_service_registration_tests.rs"]
mod tests;
