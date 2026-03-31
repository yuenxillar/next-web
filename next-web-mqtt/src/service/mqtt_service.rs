use next_web_core::async_trait;
use rumqttc::{ClientError, QoS};

#[async_trait]
pub trait MQTTService
where
    Self: Send + Sync,
    Self: 'static,
{
    async fn publish<S, V>(&self, topic: S, message: V) -> Result<(), ClientError>
    where
        S: Into<String> + Send,
        V: Into<Vec<u8>> + Send;

    async fn publish_with_qos<S, V>(
        &self,
        topic: S,
        qos: QoS,
        message: V,
    ) -> Result<(), ClientError>
    where
        S: Into<String> + Send,
        V: Into<Vec<u8>> + Send;

    async fn publish_with_retain<S, V>(
        &self,
        topic: S,
        qos: QoS,
        retain: bool,
        message: V,
    ) -> Result<(), ClientError>
    where
        S: Into<String> + Send,
        V: Into<Vec<u8>> + Send;
}
