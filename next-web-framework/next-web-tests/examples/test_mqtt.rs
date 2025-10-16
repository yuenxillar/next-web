use axum::response::IntoResponse;
use axum::Json;
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::application::Application;
use next_web_dev::extract::find_singleton::FindSingleton;
use next_web_dev::Singleton;
use next_web_mqtt::core::topic::base_topic::BaseTopic;
use next_web_mqtt::service::mqtt_service::MQTTService;

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().route("/publish", axum::routing::post(publish_message))
    }
}

async fn publish_message(
    FindSingleton(mqtt_service): FindSingleton<MQTTService>,
    Json(msg): Json<String>,
) -> impl IntoResponse {
    let topic = "test/publish";
    mqtt_service.publish(topic, msg).await.ok();
    "Ok"
}

#[Singleton( binds = [Self::into_base_topic])]
#[derive(Clone)]
pub(crate) struct TestOneBaseTopic;

impl TestOneBaseTopic {
    fn into_base_topic(self) -> Box<dyn BaseTopic> {
        Box::new(self)
    }
}

#[Singleton( binds = [Self::into_base_topic])]
#[derive(Clone)]
pub(crate) struct TestTwoBaseTopic;

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
            "Received message1, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}

#[async_trait]
impl BaseTopic for TestTwoBaseTopic {
    fn topic(&self) -> &'static str {
        "test/123"
    }

    async fn consume(&self, topic: &str, message: &[u8]) {
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
