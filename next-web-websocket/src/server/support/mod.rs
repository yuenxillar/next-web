/// Validates Origin header during WebSocket handshake to prevent CSWSH attacks
pub mod origin_handshake_interceptor;

/// WebSocket connection context containing session metadata and state
pub mod ws_context;

/// Routes WebSocket connections to handlers based on URL path patterns
pub mod ws_handler_mapping;

/// Manages individual WebSocket connection lifecycle and session state
pub mod ws_session;
