use std::{error::Error, sync::Arc};

use redis::Client;

use crate::{
    connection::default_message::DefaultMessage,
    listener::{
        key_expiration_event_message_listener::KeyExpirationEventMessageListener, topic::Topic,
    },
};

#[derive(Clone)]
pub struct RedisMessageListenerContainer {
    client: Arc<Client>,
}

impl RedisMessageListenerContainer {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }

    /// Add message listener for Redis expired-key events.
    pub async fn add_message_listener<T>(
        &mut self,
        message_listener: Arc<dyn KeyExpirationEventMessageListener>,
        topic: T,
    ) -> Result<(), Box<dyn Error>>
    where
        T: Topic,
    {
        use futures::StreamExt;
        use redis::Value;

        let (mut sink, mut stream) = self.client.get_async_pubsub().await?.split();
        sink.psubscribe(topic.get_topic()).await?;

        ::tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                let payload = match msg.get_payload() {
                    Ok(Value::BulkString(bytes)) => bytes,
                    _ => continue,
                };
                let channel = match msg.get_channel() {
                    Ok(Value::BulkString(bytes)) => bytes,
                    _ => continue,
                };

                let default_message = DefaultMessage::new(channel, payload);

                let pattern = match msg.get_pattern() {
                    Ok(Value::BulkString(pattern)) => pattern,
                    _ => Default::default(),
                };

                message_listener
                    .on_message(&default_message, &pattern)
                    .await;
            }
        });

        Ok(())
    }
}
