use std::{error::Error, sync::Arc};

use axum::extract::ws::CloseFrame;
use next_web_core::async_trait;
use next_web_dev::Singleton;
use next_web_websocket::{core::handler::WebSocketHandler, models::session::WebSocketSession, Message};
use next_web_websocket::core::handler::Result;

#[Singleton(binds = [Self::into_websocket_handler])]
#[derive(Clone)]
pub(crate) struct TestWebSocket;

impl TestWebSocket {
    fn into_websocket_handler(self) -> Arc<dyn WebSocketHandler> {
        Arc::new(self)
    }
}

#[async_trait]
impl WebSocketHandler for TestWebSocket {
    fn paths(&self) -> Vec<&'static str> {
        vec!["/test1/websocket", "/test1/websocket2"]
    }

    // When the socket connection enters, this method will be entered first
    async fn on_open(&self, session: &WebSocketSession) -> Result<()> {
        println!(
            "Client remote address: {:?}, Session id: {:?}, Client header: {:?}, Client paths: {:?}",
            session.remote_address(),
            session.id(),
            session.header(),
            session.path());
        Ok(())
    }

    /// When the client sends a message, it will enter the following method
    async fn on_message(&self, _session: &WebSocketSession, message: Message) -> Result<()> {
        if let Message::Text(msg) = message {
            println!("Received message: {:?}", msg.to_string());
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

