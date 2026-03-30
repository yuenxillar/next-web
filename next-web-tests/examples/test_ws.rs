//! Test websocket handler

use std::error::Error;
use std::sync::Arc;

use axum::extract::ws::CloseFrame;
use next_web::application::Application;
use next_web::{ApplicationContext, async_trait, macros::bind::singleton};
use next_web_core::context::properties::ApplicationProperties;
use next_web_websocket::ws::Message;
use next_web_websocket::ws::config::ws_configurer::WebSocketConfigurer;
use next_web_websocket::ws::config::ws_handler_registry::WebSocketHandlerRegistry;
use next_web_websocket::ws::server::support::ws_session::WebSocketSession;
use next_web_websocket::ws::ws_handler::{WSResult, WebSocketHandler};

#[singleton(binds = [Self::into_websocket_configurer])]
#[derive(Clone)]
pub struct TestWebSocketConfigurer;

impl WebSocketConfigurer for TestWebSocketConfigurer {
    fn register_web_socket_handlers(
        &mut self,
        _ctx: &mut ApplicationContext,
        registry: &mut dyn WebSocketHandlerRegistry,
    ) {
        registry.add_handler(Arc::new(TestWSHandler), vec!["/ws".into()]);
    }
}

impl TestWebSocketConfigurer {
    fn into_websocket_configurer(self) -> Box<dyn WebSocketConfigurer> {
        Box::new(self)
    }
}

impl TestWSHandler {
    fn into_websocket_handler(self) -> Arc<dyn WebSocketHandler> {
        Arc::new(self)
    }
}

#[singleton(binds = [Self::into_websocket_handler])]
#[derive(Clone)]
pub struct TestWSHandler;

#[async_trait]
impl WebSocketHandler for TestWSHandler {
    // When the socket connection enters, this method will be entered first
    async fn on_open(&self, session: &WebSocketSession) -> WSResult<()> {
        println!(
            "Client remote address: {:?}, session id: {:?}",
            session.remote_address(),
            session.id()
        );
        Ok(())
    }

    /// When the client sends a message, it will enter the following method
    async fn on_message(&self, _session: &WebSocketSession, message: Message) -> WSResult<()> {
        if let Message::Text(msg) = message {
            println!("User message: {}", msg.to_string());
        }
        Ok(())
    }

    /// When an error occurs during the connection process or message transmission, the following methods will be executed
    async fn on_error(
        &self,
        _session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> WSResult<()> {
        println!("On error: {:#}", error);
        Ok(())
    }

    /// After handling the error, close the connection and proceed to the following method
    async fn on_close(
        &self,
        session: &WebSocketSession,
        _close: Option<CloseFrame>,
    ) -> WSResult<()> {
        println!("User left: {:?}", session.id());
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct TestWSApplication;

#[async_trait]
impl Application for TestWSApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) {
    }
}

#[tokio::main]
async fn main() {
    TestWSApplication::run().await;
}
