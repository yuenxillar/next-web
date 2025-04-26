# Next Web MQTT

MQTT- make everything simpler

If you want to use it, please ensure that the file contains the following content

 // CARGO_MANIFEST_DIR/resources/application.yaml

And lib

next-web-dev  

# Used in conjunction, otherwise useless

-----------------------------------------
next:
    mqtt:
        client_id: test-id
        host: localhost
        port: 1883
        username: test
        password: test
        topics: test/#,test2/#
        keep_alive: 5000
        clean_session: true

-------------------------------------------

```rust
use next_web_dev::{SingleOwner, Singleton};
use next_web_core::async_trait;

use next_web_mqtt::{core::topic::base_topic::BaseTopic, service::mqtt_service::MQTTService};

// use async_trait::async_trait


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
   
    // 
}

```