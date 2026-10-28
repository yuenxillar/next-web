//! Runnable SockJS server with WebSocket and HTTP fallback transports.
//!
//! Run with `cargo run -p next-web-websocket --example sockjs_example`, then
//! connect a browser SockJS client to `http://127.0.0.1:8080/events`.

use std::{error::Error, sync::Arc};

use axum::extract::ws::{CloseFrame, Message};
use next_web_core::async_trait;
use next_web_websocket::{
    config::SockJsServiceRegistration,
    server::support::ws_session::WebSocketSession,
    sockjs::{router::SockJsRouter, service::SockJsService},
    ws_handler::{WSResult, WebSocketHandler},
};

struct EchoHandler;

#[async_trait]
impl WebSocketHandler for EchoHandler {
    async fn on_open(&self, session: &WebSocketSession) -> WSResult<()> {
        session
            .send_message(Message::Text("Connected through SockJS".into()))
            .await
            .map_err(|error| format!("failed to send welcome message: {error:?}"))?;
        Ok(())
    }

    async fn on_message(&self, session: &WebSocketSession, message: Message) -> WSResult<()> {
        if let Message::Text(text) = message {
            session
                .send_message(Message::Text(format!("Echo: {text}").into()))
                .await
                .map_err(|error| format!("failed to send echo: {error:?}"))?;
        }
        Ok(())
    }

    async fn on_error(
        &self,
        _session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> WSResult<()> {
        eprintln!("SockJS handler error: {error}");
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut registration = SockJsServiceRegistration::default();
    registration
        .set_websocket_enabled(true)
        .set_heartbeat_time(25_000)
        .set_allowed_origin_patterns(vec!["http://localhost:*".to_string()]);

    let service: Arc<dyn SockJsService> =
        Arc::from(registration.get_sockjs_service_with_application_handler(Arc::new(EchoHandler)));
    let app = axum::Router::new().sockjs("/events", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;

    println!("SockJS listening at http://127.0.0.1:8080/events");
    println!("Available transports: WebSocket, XHR streaming, XHR polling");
    axum::serve(listener, app).await?;
    Ok(())
}
