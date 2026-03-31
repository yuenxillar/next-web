use std::sync::Arc;

use axum::Json;
use axum::response::IntoResponse;
use next_web::application::Application;
use next_web::extract::find_singleton::FindSingleton;
use next_web::macros::bind::singleton;
use next_web_core::{ApplicationContext, async_trait, context::properties::ApplicationProperties};
use next_web_mqtt::{
    service::default_mqtt_service::DefaultMQTTService, topic_listener::TopicListener,
};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) {
    }

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().route("/publish", axum::routing::post(publish_message))
    }
}

async fn publish_message(
    FindSingleton(mqtt_service): FindSingleton<DefaultMQTTService>,
    Json(msg): Json<String>,
) -> impl IntoResponse {
    let topic = "test/1/event";
    mqtt_service.publish(topic, msg).await.ok();
    "Ok"
}

#[singleton( binds = [Self::into_topic_listener])]
#[derive(Clone)]
pub(crate) struct TestOneTopicListener;

impl TestOneTopicListener {
    fn into_topic_listener(self) -> Arc<dyn TopicListener> {
        Arc::new(self)
    }
}

#[singleton( binds = [Self::into_topic_listener])]
#[derive(Clone)]
pub(crate) struct TestTwoTopicListener;

impl TestTwoTopicListener {
    fn into_topic_listener(self) -> Arc<dyn TopicListener> {
        Arc::new(self)
    }
}

#[async_trait]
impl TopicListener for TestOneTopicListener {
    fn topic(&self) -> &'static str {
        "test/+/event"
    }

    async fn on_message(&self, topic: &str, message: &[u8]) {
        println!(
            "Received message1, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}

#[async_trait]
impl TopicListener for TestTwoTopicListener {
    fn topic(&self) -> &'static str {
        "test/123"
    }

    async fn on_message(&self, topic: &str, message: &[u8]) {
        println!(
            "Received message2, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
