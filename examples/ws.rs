use std::{collections::HashMap, error::Error, sync::Arc};

use next_web::core::any::AnyValue;
use next_web::core::error::BoxError;
use next_web::core::traits::http::http_request::HttpRequest;
use next_web::core::traits::http::http_response::HttpResponse;
use next_web::core::{ApplicationContext, async_trait};
use next_web::macros::bind::singleton;
use next_web::{Application, NextWebApplication};

use next_web::ws::config::ws_configurer::WebSocketConfigurer;
use next_web::ws::config::ws_handler_registry::WebSocketHandlerRegistry;
use next_web::ws::server::handshake_interceptor::HandshakeInterceptor;
use next_web::ws::server::support::ws_session::WebSocketSession;
use next_web::ws::ws_handler::{WSResult, WebSocketHandler};
use next_web::ws::{CloseFrame, Message};

#[derive(Default)]
pub struct MyApplication;

impl Application for MyApplication {}

#[singleton(binds = [Self::into_websocket_handler])]
#[derive(Clone)]
pub struct EchoHandler;

impl EchoHandler {
    fn into_websocket_handler(self) -> Arc<dyn WebSocketHandler> {
        Arc::new(self)
    }
}

#[async_trait]
impl WebSocketHandler for EchoHandler {
    async fn on_open(&self, session: &WebSocketSession) -> WSResult<()> {
        println!(
            "connected: {} from {}",
            session.id(),
            session.remote_address()
        );
        session
            .send_message(Message::Text("welcome".into()))
            .await
            .map_err(|error| format!("the client is already gone: {error:?}"))?;
        Ok(())
    }

    async fn on_message(&self, session: &WebSocketSession, message: Message) -> WSResult<()> {
        // Every frame arrives here, including Ping, Pong and Close, so match on
        // the variant rather than assuming it is text.
        if let Message::Text(text) = message {
            session
                .send_message(Message::Text(format!("echo: {text}").into()))
                .await
                .map_err(|error| format!("could not reply: {error:?}"))?;
        }
        Ok(())
    }

    async fn on_error(
        &self,
        _session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> WSResult<()> {
        eprintln!("websocket error: {error}");
        Ok(())
    }

    async fn on_close(
        &self,
        session: &WebSocketSession,
        close: Option<CloseFrame>,
    ) -> WSResult<()> {
        println!("disconnected: {} ({close:?})", session.id());
        Ok(())
    }
}

// -- 2. the handshake interceptor (optional) --------------------------------

#[singleton(binds = [Self::into_interceptor])]
#[derive(Clone)]
pub struct AuthInterceptor;

impl AuthInterceptor {
    fn into_interceptor(self) -> Arc<dyn HandshakeInterceptor> {
        Arc::new(self)
    }
}

#[async_trait]
impl HandshakeInterceptor for AuthInterceptor {
    async fn before_handshake(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _attributes: &mut HashMap<String, AnyValue>,
    ) -> Result<bool, BoxError> {
        // `Ok(false)` refuses the upgrade and the response below is what the
        // client receives. Returning `true` continues to the handshake.
        match request.header("authorization") {
            Some(_) => Ok(true),
            None => {
                response.insert_header("x-reason", "missing authorization");
                Ok(false)
            }
        }
    }
}

// -- 3. the configurer: which paths, which origins, SockJS or not -----------

#[singleton(binds = [Self::into_websocket_configurer])]
#[derive(Clone)]
pub struct MyWebSocketConfigurer;

impl MyWebSocketConfigurer {
    fn into_websocket_configurer(self) -> Box<dyn WebSocketConfigurer> {
        Box::new(self)
    }
}

impl WebSocketConfigurer for MyWebSocketConfigurer {
    fn register_websocket_handlers(
        &mut self,
        _ctx: &mut dyn ApplicationContext,
        registry: &mut dyn WebSocketHandlerRegistry,
    ) {
        registry
            .add_handler(Arc::new(EchoHandler), vec!["/ws".into()])
            .set_allowed_origin_patterns(vec!["https://*.example.com".into()]);
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<MyApplication>::default().run().await;
}
