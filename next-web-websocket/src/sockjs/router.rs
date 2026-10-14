//! Axum router integration for SockJS endpoints.
//!
//! Provides utilities to mount SockJS services onto Axum routers
//! with the standard SockJS URL structure.

use std::sync::Arc;

use axum::{
    Router,
    extract::{ConnectInfo, Path, Request, State, ws::WebSocketUpgrade},
    response::IntoResponse,
    response::Response,
    routing::{any, get},
};
use std::net::SocketAddr;

use crate::sockjs::service::SockJsService;

/// Type alias for a router with SockJS state.
pub type SockJsRouterType = Router<Arc<dyn SockJsService>>;

/// Extension trait for adding SockJS routes to an Axum router.
pub trait SockJsRouter {
    /// Adds SockJS routes to the router.
    ///
    /// This mounts the following endpoints:
    /// - `GET /info` - Server capabilities
    /// - `GET /{server-id}/{session-id}/{transport}` - Transport endpoints
    /// - `POST /{server-id}/{session-id}/{transport}` - Send messages
    ///
    /// # Arguments
    ///
    /// * `path` - Base path for SockJS endpoints (e.g., "/sockjs")
    /// * `service` - SockJS service implementation
    fn sockjs(self, path: &str, service: Arc<dyn SockJsService>) -> Self;
}

impl SockJsRouter for Router<()> {
    fn sockjs(self, path: &str, service: Arc<dyn SockJsService>) -> Self {
        // Construct paths with Axum 0.8 syntax: {param} instead of :param
        let base_path = path.trim_end_matches('/');
        let info_path = format!("{}/info", base_path);
        let websocket_path = format!("{}/{{server}}/{{session}}/websocket", base_path);
        let transport_path = format!("{}/{{server}}/{{session}}/{{transport}}", base_path);

        let sockjs_router = Router::new()
            .route(&info_path, any(handle_transport))
            .route(&websocket_path, get(handle_websocket))
            .route(&transport_path, any(handle_transport))
            .with_state(service);

        self.merge(sockjs_router)
    }
}

/// Handles the native SockJS WebSocket transport.
async fn handle_websocket(
    State(service): State<Arc<dyn SockJsService>>,
    Path((_server, session_id)): Path<(String, String)>,
    ws: WebSocketUpgrade,
    mut request: Request,
) -> Response {
    if let Some(response) = match service.authorize_websocket_request(&mut request).await {
        Ok(response) => response,
        Err(error) => {
            tracing::error!("SockJS WebSocket authorization failed: {}", error);
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    } {
        return response;
    }

    let headers = request.headers().clone();
    let remote_addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0)
        .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 0)));

    match service
        .handle_websocket_upgrade(ws, session_id, headers, remote_addr)
        .await
    {
        Ok(mut response) => {
            service.add_cors_headers(&request, &mut response);
            response
        }
        Err(error) => {
            tracing::error!("SockJS WebSocket upgrade failed: {}", error);
            axum::http::Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from("SockJS WebSocket upgrade failed"))
                .expect("valid error response")
        }
    }
}

/// Handles requests to transport endpoints.
async fn handle_transport(State(service): State<Arc<dyn SockJsService>>, req: Request) -> Response {
    match service.handle_request(req).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("SockJS request failed: {}", e);
            axum::http::Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from(format!("Error: {}", e)))
                .unwrap()
        }
    }
}
