# Next Web MQTT

MQTT- make everything simpler

If you want to use it, please ensure that the file contains the following content

 // CARGO_MANIFEST_DIR/resources/application.yaml

And lib

next-web-dev

# Used in conjunction, otherwise useless

```yaml

next:
    mqtt:
        host: localhost
        port: 1883
        username: user1
        password: 123
        topics:
            - test/#
            - testtopic/#
            # with qos 
            # 默认匹配最后两个字符 希望是以下字符-> :0 :1 :2 
            # 如果都没有匹配, QOS 消息质量默认为 1
            - test/666:0
        # from secs
        connect_timeout: 10
        clean_session: true

```

```rust
#![allow(missing_docs)]

use next_web_core::async_trait;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application,
    Singleton,
};
use next_web_mqtt::{core::topic::base_topic::BaseTopic, service::mqtt_service::MQTTService};

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
    ) -> axum::Router {
        let mqtt = ctx.get_single_with_name::<MQTTService>("mqttService");
        mqtt.publish("test/two", "hello world!").await.unwrap();
        axum::Router::new()
    }
}

#[Singleton(binds = [Self::into_base_topic])]
#[derive(Clone)]
pub struct TestOneBaseTopic;

impl TestOneBaseTopic {
    fn into_base_topic(self) -> Box<dyn BaseTopic> {
        Box::new(self)
    }
}
 
#[Singleton(binds = [Self::into_base_topic])]
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
#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

```