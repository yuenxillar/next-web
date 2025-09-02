use axum::response::IntoResponse;
use axum::Json;
use next_web_core::AutoRegister;
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::application::Application;
use next_web_dev::middleware::find_singleton::FindSingleton;
use next_web_dev::Singleton;
use next_web_mqtt::auto_register::mqtt_service_auto_register::MQTTServiceAutoRegister;
use next_web_mqtt::core::topic::base_topic::BaseTopic;
use next_web_mqtt::properties::mqtt_properties::MQTTClientProperties;
use next_web_mqtt::service::mqtt_service::MQTTService;

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {
        let mut properties = MQTTClientProperties::default();
        properties.set_host("127.0.0.1");
        properties.set_username("username");
        properties.set_password("password");
        properties.set_topics(vec!["test/#".into()]);

        let auto = MQTTServiceAutoRegister(properties);
        auto.register(ctx, &Default::default()).await.ok();

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
