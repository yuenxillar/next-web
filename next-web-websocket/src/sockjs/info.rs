//! SockJS /info endpoint implementation.
//!
//! The /info endpoint provides client capability negotiation, telling
//! the client what transport mechanisms are available.

use axum::{
    body::Body,
    http::{StatusCode, header},
    response::Response,
};
use serde::Serialize;

/// Information returned by the /info endpoint.
///
/// This tells the client which transport mechanisms are available
/// and other server capabilities.
#[derive(Debug, Clone, Serialize)]
pub struct SockJsInfo {
    /// Whether WebSocket transport is enabled.
    pub websocket: bool,

    /// Whether the server requires cookies (JSESSIONID for load balancing).
    pub cookie_needed: bool,

    /// List of allowed origins (CORS).
    pub origins: Vec<String>,

    /// Random entropy value (for client use).
    pub entropy: u32,
}

impl SockJsInfo {
    /// Creates a new SockJS info response.
    ///
    /// # Arguments
    ///
    /// * `websocket_enabled` - Whether WebSocket transport is available
    /// * `cookie_needed` - Whether session cookies are required
    pub fn new(websocket_enabled: bool, cookie_needed: bool) -> Self {
        Self {
            websocket: websocket_enabled,
            cookie_needed,
            origins: vec!["*:*".to_string()],
            entropy: rand::random(),
        }
    }

    /// Converts the info to an HTTP response.
    pub fn to_response(&self) -> Response {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json; charset=UTF-8")
            .header(
                header::CACHE_CONTROL,
                "no-store, no-cache, no-transform, must-revalidate, max-age=0",
            )
            .body(Body::from(json))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sockjs_info_creation() {
        let info = SockJsInfo::new(true, true);
        assert!(info.websocket);
        assert!(info.cookie_needed);
        assert_eq!(info.origins, vec!["*:*"]);
    }

    #[test]
    fn test_sockjs_info_serialization() {
        let info = SockJsInfo {
            websocket: true,
            cookie_needed: false,
            origins: vec!["*:*".to_string()],
            entropy: 12345,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"websocket\":true"));
        assert!(json.contains("\"cookie_needed\":false"));
        assert!(json.contains("\"entropy\":12345"));
    }
}
