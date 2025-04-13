use crate::{
    core::topic::base_topic::BaseTopic, properties::mqtt_properties::MQTTClientProperties,
};

use hashbrown::HashMap;
use next_web_core::core::service::Service;
use rumqttc::{AsyncClient, ConnectReturnCode, Event, MqttOptions, Packet, QoS};
use tracing::error;

/// 邮件 Service
#[derive(Clone)]
pub struct MQTTService {
    properties: MQTTClientProperties,
    client: AsyncClient,
}

impl Service for MQTTService {}

impl MQTTService {
    pub fn new(properties: MQTTClientProperties, base_topics: HashMap<String,Box<dyn BaseTopic>>) -> Self {
        let client = Self::build_client(&properties, base_topics);
        Self { properties, client }
    }

    fn build_client(
        properties: &MQTTClientProperties,
        mut base_topics: HashMap<String, Box<dyn BaseTopic>>
    ) -> AsyncClient {
        let mut options  = MqttOptions::new(
            properties.client_id().unwrap_or("Next-Web-Client-ID"),
            properties.host().unwrap_or("localhost"),
            properties.port().unwrap_or(1883),
        );

        options
            .set_keep_alive(std::time::Duration::from_secs(
                properties.keep_alive().unwrap_or(5),
            ))
            .set_clean_session(properties.clean_session().unwrap_or(false))
            .set_credentials(
                properties.username().unwrap_or_default(),
                properties.password().unwrap_or_default(),
            );

        let (client, mut eventloop) = AsyncClient::new(options, 666);

        let topics = properties.topics();
        let client_1 = client.clone();

        
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(packet))) => {
                        let data = packet.payload;
                        let topic = packet.topic;
                        if let Some(basic) = base_topics.get_mut(&topic) {
                            basic.consume(&topic, &data).await;
                        }
                    }

                    Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                        // 这里可以接受 ack 信息 重新订阅 topic
                        match ack.code {
                            ConnectReturnCode::Success => {
                                for topic in topics.iter() {
                                    client_1.subscribe(topic, QoS::AtLeastOnce).await.unwrap();
                                }
                            }
                            _ => {}
                        }
                    }

                    Err(e) => {
                        error!("Mqtt Eventloop Pool Error, ConnectionError Case: {:?}", e);
                    }

                    _ => {
                        // dodo -> Outgoing
                    }
                }
            }
        });

        client
    }

    pub async fn publish<M: Into<Vec<u8>>>(&self, topic: &str, message: M) {
        let _ = self
            .client
            .publish(topic, QoS::AtLeastOnce, false, message)
            .await;
    }

    pub async fn publish_and_qos<M: Into<Vec<u8>>>(&self, topic: &str, q: u8, message: M) {
        if let Ok(qos) = rumqttc::qos(q) {
            let _ = self.client.publish(topic, qos, false, message).await;
        }
    }

    pub fn get_client(&self) -> &AsyncClient {
        &self.client
    }

    pub fn properties(&self) -> &MQTTClientProperties {
        &self.properties
    }
}
