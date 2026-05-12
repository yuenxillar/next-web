use std::sync::Arc;

use crate::{
    server::handshake_interceptor::HandshakeInterceptor, ws_handler::WebSocketHandler,
};

/// Provides methods for configuring a WebSocket handler.
pub trait WebSocketHandlerRegistration {
    /// Add more handlers that will share the same configuration (interceptors, SockJS
    //  config, etc).
    fn add_handler(
        &mut self,
        handler: Arc<dyn WebSocketHandler>,
        paths: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration;

    /// Configure interceptors for the handshake request.
    fn add_interceptors(
        &mut self,
        interceptors: Vec<Arc<dyn HandshakeInterceptor>>,
    ) -> &mut dyn WebSocketHandlerRegistration;

    /// Set the origins for which cross-origin requests are allowed from a browser.
    /// Please, refer to {@link CorsConfiguration#setAllowedOrigins(List)} for
    /// format details and considerations, and keep in mind that the CORS spec
    fn set_allowed_origins(
        &mut self,
        origins: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration;

    /// Alternative to {@link #setAllowedOrigins(String...)} that supports more
    /// flexible patterns for specifying the origins for which cross-origin
    /// requests are allowed from a browser.
    fn set_allowed_origin_patterns(
        &mut self,
        origin_patterns: Vec<String>,
    ) -> &mut dyn WebSocketHandlerRegistration;
}
