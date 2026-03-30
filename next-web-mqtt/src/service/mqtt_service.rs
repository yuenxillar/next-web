use std::ops::Deref;

use crate::{
    core::{
        interceptor::message_interceptor::MessageInterceptor, route::TopicRoute,
        topic::base_topic::BaseTopic,
    },
    properties::mqtt_properties::MQTTClientProperties,
};

use hashbrown::HashMap;
use next_web_core::{error::BoxError, impl_service};
use rumqttc::{
    AsyncClient, ConnectReturnCode, Event, MqttOptions, NetworkOptions, Packet, QoS,
    SubscribeFilter,
};
use tracing::{error, warn};

/// MQTT service.
///
/// This service is responsible for:
/// - configuring the MQTT client
/// - subscribing configured topics
/// - dispatching published messages to exact routes and wildcard routes
/// - invoking message interceptors before consumption
#[derive(Clone)]
pub struct MQTTService {
    /// MQTT client configuration properties.
    properties: MQTTClientProperties,
    /// Async MQTT client instance.
    client: AsyncClient,
}

impl MQTTService {
    /// Creates a new `MQTTService`.
    pub fn new(
        properties: MQTTClientProperties,
        route_map: HashMap<String, Box<dyn BaseTopic>>,
        route: Vec<TopicRoute>,
        interceptor: Box<dyn MessageInterceptor>,
    ) -> Result<Self, BoxError> {
        let client = Self::build_client(&properties, route_map, route, interceptor)?;
        Ok(Self { properties, client })
    }

    /// Builds and configures the MQTT client, subscriptions and event loop.
    fn build_client(
        properties: &MQTTClientProperties,
        mut route_map: HashMap<String, Box<dyn BaseTopic>>,
        mut route: Vec<TopicRoute>,
        interceptor: Box<dyn MessageInterceptor>,
    ) -> Result<AsyncClient, BoxError> {
        let mut options = MqttOptions::new(
            properties.client_id().unwrap_or_default(),
            properties.host().unwrap_or("127.0.0.1"),
            properties.port().unwrap_or(1883),
        );

        options
            .set_keep_alive(std::time::Duration::from_millis(
                properties.keep_alive().unwrap_or(60_000),
            ))
            .set_clean_session(properties.clean_session().unwrap_or(true))
            .set_credentials(
                properties.username().unwrap_or_default(),
                properties.password().unwrap_or_default(),
            );

        let (client, mut eventloop) = AsyncClient::new(options, 999);

        let mut network_options = NetworkOptions::new();
        network_options.set_connection_timeout(properties.connect_timeout().unwrap_or(10));
        eventloop.set_network_options(network_options);

        let topics = properties.topics();
        let reconnect_client = client.clone();

        let subscribe_filters = topics
            .iter()
            .map(|topic| {
                let qos = topic.qos.map(resolve_qos).unwrap_or(QoS::AtLeastOnce);
                SubscribeFilter::new(topic.topic.clone(), qos)
            })
            .collect::<Vec<_>>();
        client.try_subscribe_many(subscribe_filters)?;

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(packet))) => {
                        let message = packet.payload;
                        let topic = packet.topic;

                        if !interceptor.message_entry(&topic, &message).await {
                            continue;
                        }

                        if let Some(exact_topic) = route_map.get_mut(&topic) {
                            exact_topic.consume(&topic, &message).await;
                        }

                        for wildcard_route in route.iter_mut() {
                            if wildcard_route.matches(&topic) {
                                wildcard_route.base_topic.consume(&topic, &message).await;
                            }
                        }
                    }

                    Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                        if ack.code == ConnectReturnCode::Success {
                            for topic in topics.iter() {
                                if let Err(err) = reconnect_client
                                    .subscribe(
                                        topic.topic.clone(),
                                        topic.qos.map(resolve_qos).unwrap_or(QoS::AtLeastOnce),
                                    )
                                    .await
                                {
                                    error!(
                                        "Failed to resubscribe topic {} after reconnect: {:?}",
                                        topic.topic, err
                                    );
                                }
                            }
                            warn!(
                                "Client reconnected successfully, resubscribed configured topics"
                            );
                        }
                    }

                    Err(err) => {
                        error!("MQTT eventloop error: {:?}", err);
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }

                    _ => {
                        // Ignore other incoming or outgoing events for now.
                    }
                }
            }
        });

        Ok(client)
    }

    /// Publishes a message with default QoS `AtLeastOnce`.
    pub async fn publish<S, V>(&self, topic: S, message: V) -> Result<(), rumqttc::ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.client
            .publish(topic, QoS::AtLeastOnce, false, message)
            .await
    }

    /// Publishes a message with a custom QoS level.
    pub async fn publish_with_qos<S, V>(
        &self,
        topic: S,
        q: u8,
        message: V,
    ) -> Result<(), rumqttc::ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.client
            .publish(topic, resolve_qos(q), false, message)
            .await
    }

    /// Publishes a message with a custom QoS level and retain flag.
    pub async fn publish_with_retain<S, V>(
        &self,
        topic: S,
        q: u8,
        retain: bool,
        message: V,
    ) -> Result<(), rumqttc::ClientError>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.client
            .publish(topic, resolve_qos(q), retain, message)
            .await
    }

    /// Returns a reference to the MQTT client.
    pub fn get_client(&self) -> &AsyncClient {
        &self.client
    }

    /// Returns a reference to the MQTT properties.
    pub fn properties(&self) -> &MQTTClientProperties {
        &self.properties
    }
}

impl Deref for MQTTService {
    type Target = AsyncClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

fn resolve_qos(qos: u8) -> QoS {
    rumqttc::qos(qos).unwrap_or(QoS::AtLeastOnce)
}

impl_service!(MQTTService);
