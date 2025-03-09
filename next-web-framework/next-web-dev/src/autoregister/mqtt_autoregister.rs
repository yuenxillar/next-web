use next_web_mqtt::rumqttc::v5::mqttbytes::QoS;
use next_web_mqtt::rumqttc::v5::EventLoop;
use std::sync::Arc;
use std::time::Duration;
use futures::executor::block_on;
use tracing::{error, info};

use next_web_mqtt::core::client_controller::MQTTClientController;

use next_web_mqtt::core::client_properties::MQTTClientProperties;
use next_web_mqtt::rumqttc::v5::{AsyncClient, MqttOptions};

use crate::manager::mqtt_manager::MQTTManager;

use super::auto_register::AutoRegister;

pub struct MqttAutoregister(pub MQTTClientProperties);

impl AutoRegister for MqttAutoregister {
    fn register(&self, ctx: &mut rudi::Context) -> Result<(), Box<dyn std::error::Error>> {
        
        let (async_client, event_loop) = block_on(Self::__register(self.0.clone()));

        let handler = ctx.resolve_option::<Arc<dyn MQTTClientController>>();
        if let Some(handler) = handler {
            let manager = MQTTManager::new(async_client, self.0.clone());
            manager.eventloop(handler, event_loop);
            ctx.insert_singleton_with_name::<MQTTManager, String>(
                manager,
                String::from("mqttManager"),
            );
        }
        Ok(())
    }
}

impl MqttAutoregister {
    pub async fn __register(properties: MQTTClientProperties) -> (AsyncClient, EventLoop) {
        let mut mqttoptions = MqttOptions::new(
            properties.id().unwrap_or("rumqttc"),
            properties.host().unwrap_or("localhost"),
            properties.port().unwrap_or(1883),
        );
        mqttoptions.set_clean_start(true);
        mqttoptions.set_credentials(
            properties.username().unwrap_or_default(),
            properties.password().unwrap_or_default(),
        );
        mqttoptions.set_keep_alive(Duration::from_secs(properties.keep_alive().unwrap_or(5)));

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 66);

        // Subscribe many topics
        for topic in properties.topics() {
            if let Err(e) = client.subscribe(&topic, QoS::AtMostOnce).await {
                error!("Error while subscribing to topic: {}", e);
            } else {
                info!("Subscribed to topic: {}", &topic);
            }
        }
        (client, eventloop)
    }
}
