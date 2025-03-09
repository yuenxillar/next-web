use axum::handler;
use next_web_mqtt::rumqttc::v5::mqttbytes::v5::{ConnectReturnCode, Packet};
use next_web_mqtt::rumqttc::v5::mqttbytes::QoS;
use next_web_mqtt::rumqttc::v5::EventLoop;
use std::sync::Arc;
use tracing::{error, info};

use next_web_mqtt::core::client_controller::MQTTClientController;
use next_web_mqtt::core::client_properties::MQTTClientProperties;
use next_web_mqtt::rumqttc::v5::{AsyncClient, Event};

#[derive(Clone)]
pub struct MQTTManager {
    client: AsyncClient,
    properties: MQTTClientProperties,
}

impl MQTTManager {
    pub fn new(client: AsyncClient, properties: MQTTClientProperties) -> Self {
        Self { client, properties }
    }

    pub fn eventloop(&self, handler: Arc<dyn MQTTClientController>, mut event: EventLoop) {
        let topics = self.properties.topics();
        let client = self.client.clone();
        tokio::spawn(async move {
            while let Ok(notification) = event.poll().await {
                match notification {
                    Event::Incoming(Packet::Publish(packet)) => {
                        handler
                            .message(&String::from_utf8_lossy(&packet.topic), &packet.payload)
                            .await;
                    }

                    Event::Incoming(Packet::ConnAck(ack)) => {
                        // 这里可以接受 ack 信息 重新订阅 topic
                        match ack.code {
                            ConnectReturnCode::Success => {
                                for topic in &topics {
                                    if let Err(e) = client.subscribe(topic, QoS::AtLeastOnce).await
                                    {
                                        error!("Error while Resubscribing to topic: {}", e);
                                    } else {
                                        info!("ReSubscribed to topic: {}", &topic);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}
