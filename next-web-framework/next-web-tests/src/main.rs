#![allow(missing_docs)]
use std::any::{Any, TypeId};
use std::sync::Arc;

use axum::routing::post;
use axum::Router;
use next_web_core::async_trait;
use next_web_core::core::data_decoder::DataDecoder;
use next_web_core::core::service::Service;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};

use next_web_dev::application::Application;
use next_web_dev::event::application_event::ApplicationEvent;
use next_web_dev::event::application_event_publisher::ApplicationEventPublisher;
use next_web_dev::event::application_listener::ApplicationListener;
use next_web_dev::event::default_application_event_publisher::DefaultApplicationEventPublisher;
use next_web_dev::interceptor::request_data_interceptor::Data;
use next_web_dev::Singleton;
use serde::{Deserialize, Serialize};

// mod test_mqtt;
// mod test_redis;
// mod test_websocket;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {

        let m = ctx.resolve_with_name::<TestModule>("testModule");
        m.publisher.publish_event("", Box::new(TestEvent(100))).unwrap();
        Router::new().route("/test/789", post(test_789))
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct TestData {
    pub name: String,
    pub age: i32,
}

async fn test_789(Data(data): Data<TestData>) -> String {
    serde_json::to_string(&data).unwrap()
}

#[Singleton(binds=[Self::into_decoder])]
#[derive(Clone)]
pub struct TestDecoder;

impl TestDecoder {
    
    fn into_decoder(self) -> Arc<dyn DataDecoder> { Arc::new(self)}
}

impl DataDecoder for TestDecoder {
    fn decode(&self, data: &[u8]) -> Result<String, &'static str> {
        let d = data.iter().filter(|&&s | s != b'\\').copied()
        .collect::<Vec<_>>();
        Ok(String::from_utf8_lossy(&d).to_string())
    }
}


#[Singleton]
#[derive(Clone)]
pub struct TestModule {
    pub publisher: DefaultApplicationEventPublisher
}

impl Service for TestModule {}


#[Singleton]
#[derive(Clone)]
pub struct TestEvent(i32);

impl ApplicationEvent for  TestEvent {}

#[Singleton(binds=[Self::into_listener])]
#[derive(Clone)]
pub struct TestListener;

impl TestListener {
    
    fn into_listener(self) -> Box<dyn ApplicationListener> { Box::new(self)}
}
#[async_trait]
impl ApplicationListener for TestListener {
    fn tid(&self) -> TypeId {
        TypeId::of::<TestEvent>()
    }

    async fn on_application_event(&mut self, event: &Box<dyn ApplicationEvent>) {
        let any: &dyn Any = event.as_ref();
        let e = any.downcast_ref::<TestEvent>().unwrap();
        println!("i32: {}", e.0)
    }
}



#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
