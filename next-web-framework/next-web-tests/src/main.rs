#![allow(missing_docs)]

use std::error::Error;
use std::sync::Arc;

use axum::extract::ws::CloseFrame;
use next_web_core::async_trait;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application,
    router::{open_router::OpenRouter, private_router::PrivateRouter},
     Singleton,
};
use next_web_mqtt::{core::topic::base_topic::BaseTopic, service::mqtt_service::MQTTService};
use next_web_websocket::core::handler::WebSocketHandler;
use next_web_websocket::core::session::WebSocketSession;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(
        &mut self,
        ctx: &mut ApplicationContext,
    ) -> (OpenRouter, PrivateRouter) {
        let mqtt = ctx.get_single_with_name::<MQTTService>("mqttService");
        mqtt.publish("test/two", "hello world!").await;
        (OpenRouter::default(), PrivateRouter::default())
    }
}

#[Singleton( binds = [Self::into_base_topic])]
#[derive(Clone)]
pub struct TestOneBaseTopic;

impl TestOneBaseTopic {
    fn into_base_topic(self) -> Box<dyn BaseTopic> {
        Box::new(self)
    }
}
 
#[Singleton( binds = [Self::into_base_topic])]
#[derive(Clone)]
pub struct TestTwoBaseTopic;

impl TestTwoBaseTopic {
    fn into_base_topic(self) -> Box<dyn BaseTopic> {
        Box::new(self)
    }
}

#[async_trait]
impl BaseTopic for TestOneBaseTopic {
    fn topic(&self) -> &'static str {
        "test/+/event"
    }

    async fn consume(&self, topic: &str, message: &[u8]) {
        println!(
            "Received message0, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}

#[async_trait]
impl BaseTopic for TestTwoBaseTopic {
    fn topic(&self) -> &'static str {
        "test/#"
    }

    async fn consume(&self, topic: &str, message: &[u8]) {
        println!(
            "Received message1, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}



/// Test
#[Singleton(binds = [Self::into_websocket_handler])]
#[derive(Clone)]
pub struct TestWebSocket;


impl TestWebSocket {
    fn into_websocket_handler(self) -> Arc<dyn WebSocketHandler> {
        Arc::new(self)
    }
}

use next_web_websocket::core::handler::Result;
use next_web_websocket::Message;


#[async_trait]
impl WebSocketHandler for TestWebSocket {
    fn paths(&self) -> Vec<&'static str> {
        vec!["/test1/websocket", "/test1/websocket2"]
    }

    // When the socket connection enters, this method will be entered first
    async fn on_open(&self, session: &WebSocketSession) -> Result<()> {
        println!(
            "Client remote address: {:?}, session id: {:?}",
            session.remote_address(),
            session.id()
        );
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

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
