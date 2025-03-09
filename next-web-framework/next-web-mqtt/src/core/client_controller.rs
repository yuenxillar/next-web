use rumqttc::v5::{mqttbytes::QoS, AsyncClient};

#[async_trait::async_trait]
pub trait MQTTClientController: Send + Sync {
    async fn message(&self, topic: &str, payload: &[u8]);

    async fn publish(&self, topic: &str, message: String) {
        let _ = self
            .client()
            .publish(topic, QoS::AtLeastOnce, false, message)
            .await;
    }

    fn client(&self) -> &AsyncClient;
}