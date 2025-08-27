# Next Web WebSocket

WebSocket - make everything simpler


```rust
//! Test websocket handler

use std::error::Error;
use std::sync::Arc;

use axum::extract::ws::CloseFrame;
use next_web_core::async_trait;
use next_web_dev::Singleton;
use next_web_websocket::handler::websocket_handler::{Result, WebSocketHandler};
use next_web_websocket::Message;

use crate::models::session::WebSocketSession;

/// Test
#[Singleton(binds = [Self::into_websocket_handler])]
#[derive(Clone)]
pub struct TestWebSocket;

impl TestWebSocket {
    fn into_websocket_handler(self) -> Arc<dyn WebSocketHandler> {
        Arc::new(self)
    }
}

#[async_trait]
impl WebSocketHandler for TestWebSocket {
    fn paths(&self) -> Vec<&'static str> {
        vec!["/test/websocket", "/test/websocket2"]
    }

    // When the socket connection enters, this method will be entered first
    async fn on_open(&self, session: &WebSocketSession) -> Result<()> {
        println!(
            "Client remote address: {:?}, Session id: {:?}, Client header: {:?}, Client path: {:?}",
            session.remote_address(),
            session.id(),
            session.header(),
            session.path());
        Ok(())
    }

    /// When the client sends a message, it will enter the following method
    async fn on_message(&self, _session: &WebSocketSession, message: Message) -> Result<()> {
        if let Message::Text(msg) = message {
            println!("User message: {:?}", msg);
        }
        Ok(())
    }

    /// When an error occurs during the connection process or message transmission, the following methods will be executed
    async fn on_error(
        &self,
        _session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> Result<()> {
        println!("On error: {:#}", error);
        Ok(())
    }

    /// After handling the error, close the connection and proceed to the following method
    async fn on_close(&self, session: &WebSocketSession, _close: Option<CloseFrame>) -> Result<()> {
        println!("User left: {:?}", session.id());
        Ok(())
    }
}

```