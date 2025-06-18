use next_web_core::async_trait;
use next_web_dev::Singleton;
use next_web_mqtt::core::topic::base_topic::BaseTopic;


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
        "test/#"
    }

    async fn consume(&self, topic: &str, message: &[u8]) {
        println!(
            "Received message2, Topic: {}, Data Content: {:?}",
            topic,
            String::from_utf8_lossy(message)
        );
    }
}