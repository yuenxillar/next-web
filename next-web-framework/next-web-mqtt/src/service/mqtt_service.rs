use crate::{
    core::{
        interceptor::message_interceptor::MessageInterceptor,
        router::{MacthType, TopicRouter},
        topic::base_topic::BaseTopic,
    },
    properties::mqtt_properties::MQTTClientProperties,
};

use hashbrown::HashMap;
use next_web_core::core::service::Service;
use rumqttc::{
    AsyncClient, ConnectReturnCode, Event, MqttOptions, NetworkOptions, Packet, QoS,
    SubscribeFilter,
};
use tracing::error;

/// 邮件 Service
#[derive(Clone)]
pub struct MQTTService {
    properties: MQTTClientProperties,
    client: AsyncClient,
}

impl Service for MQTTService {
    fn service_name(&self) -> String {
        "mqttService".into()
    }
}

impl MQTTService {
    pub fn new(
        properties: MQTTClientProperties,
        router_map: HashMap<String, Box<dyn BaseTopic>>,
        router: Vec<TopicRouter>,
        interceptor: Box<dyn MessageInterceptor>,
    ) -> Self {
        let client = Self::build_client(&properties, router_map, router, interceptor);
        Self { properties, client }
    }

    fn build_client(
        properties: &MQTTClientProperties,
        mut base_topics: HashMap<String, Box<dyn BaseTopic>>,
        mut router: Vec<TopicRouter>,
        interceptor: Box<dyn MessageInterceptor>,
    ) -> AsyncClient {
        let mut options = MqttOptions::new(
            properties.client_id().unwrap_or("next-web-mqtt"),
            properties.host().unwrap_or("127.0.0.1"),
            properties.port().unwrap_or(1883),
        );

        options
            .set_keep_alive(std::time::Duration::from_millis(
                properties.keep_alive().unwrap_or(60000),
            ))
            .set_clean_session(properties.clean_session().unwrap_or(true))
            .set_credentials(
                properties.username().unwrap_or_default(),
                properties.password().unwrap_or_default(),
            );

        let (client, mut eventloop) = AsyncClient::new(options, 999);

        let mut network_options = NetworkOptions::new();
        network_options.set_connection_timeout(properties.connect_timeout().unwrap_or(5));
        eventloop.set_network_options(network_options);


        let topics = properties.topics();
        let client_1 = client.clone();

        // subscribe topics
        let var = topics
            .iter()
            .map(|item| SubscribeFilter::new(item.clone(), QoS::AtLeastOnce))
            .collect::<Vec<_>>();
        client.try_subscribe_many(var).unwrap();

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(packet))) => {
                        let message = packet.payload;
                        let topic = packet.topic;
                        interceptor.message_entry(&topic, &message).await;

                        if let Some(basic) = base_topics.get_mut(&topic) {
                            basic.consume(&topic, &message).await;
                        }

                        for item in router.iter_mut() {
                            match item.match_type {
                                MacthType::Anything => {
                                    item.base_topic.consume(&topic, &message).await;
                                }

                                MacthType::Multilayer(index) => {
                                    if topic[0..index].eq(&item.topic[0..index]) {
                                        item.base_topic.consume(&topic, &message).await;
                                    }
                                }

                                MacthType::Singlelayer(left_inddex, right_index) => {
                                    if topic[0..left_inddex].eq(&item.topic[0..left_inddex]) {
                                        if right_index != 0 {
                                            if !topic[right_index..].eq(&item.topic[right_index..])
                                            {
                                                continue;
                                            }
                                        }
                                        item.base_topic.consume(&topic, &message).await;
                                    }
                                }
                            }
                        }
                    }

                    Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                        // This generally refers to the need to receive ack information and re subscribe to the topic after reconnection
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
                        error!("Mqtt eventloop error, connection error case: {:?}", e);
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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
