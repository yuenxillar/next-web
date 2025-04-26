#![allow(missing_docs)]

use next_web_dev::{ application::Application, router::{open_router::OpenRouter, private_router::PrivateRouter}};
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, ApplicationContext
};
use axum::routing::get;
// use async_trait::async_trait
use next_web_dev::{SingleOwner, Singleton};
use next_web_mqtt::{core::topic::base_topic::BaseTopic, service::mqtt_service::MQTTService};

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;


#[async_trait]
impl Application for TestApplication {
    async fn init_middleware(&mut self, properties: &ApplicationProperties) {}

    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> (OpenRouter, PrivateRouter) {
        (OpenRouter::default(), PrivateRouter(axum::Router::new().route("/test", get(test_fn))))
    }
}

async fn test_fn() -> &'static str {
    " Hello Axum! \n Hello Next Web!"
}


#[SingleOwner(binds = [Self::into_base_topic])]
#[derive(Clone)]
pub struct TestBaseTopic;

impl TestBaseTopic {

    fn into_base_topic(self) -> Box<dyn BaseTopic> {
        Box::new(self)
    }
}

#[SingleOwner(binds = [Self::into_base_topic])]
#[derive(Clone)]
pub struct TestTwoBaseTopic;


impl TestTwoBaseTopic {

    fn into_base_topic(self) -> Box<dyn BaseTopic> {
        Box::new(self)
    }
}

#[async_trait]
impl BaseTopic for TestBaseTopic {

    fn topic(&self) -> &'static str {
        "test/hello"
    }

    async fn consume(&mut self, topic: &str, message: &[u8]) {
        println!("Received message, Topic: {}, Data Content: {:?}", topic,  String::from_utf8_lossy(message));
    }
}

#[async_trait]
impl BaseTopic for TestTwoBaseTopic {

    fn topic(&self) -> &'static str {
        "test/two"
    }

    async fn consume(&mut self, topic: &str, message: &[u8]) {
        println!("Received message, Topic: {}, Data Content: {:?}", topic,  String::from_utf8_lossy(message));
    }
}


#[Singleton( name = "testService")]
#[derive(Clone)]
pub struct TestService {
    #[autowired(name = "mqttService")]
    pub service: MQTTService
}


impl TestService {

    pub async fn publish(&self) {
        self.service.publish("test/publish", "hello mqtt!").await;
        self.service.publish_and_qos("test/publish", 0, "hello mqtt!").await;
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
