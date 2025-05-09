use std::ops::{Deref, DerefMut};

use futures::StreamExt;
use next_web_core::core::service::Service;
use redis::{Client, RedisError, Value, aio::MultiplexedConnection};

use crate::{
    core::event::expired_keys_event::RedisExpiredKeysEvent,
    properties::redis_properties::RedisClientProperties,
};

#[derive(Clone)]
pub struct RedisService {
    properties: RedisClientProperties,
    client: Client,
}

impl Service for RedisService {
    fn service_name(&self) -> String {
        "redisService".into()
    }
}

impl RedisService {
    pub fn new(properties: RedisClientProperties) -> Self {
        let client = Self::build_client(&properties);
        Self { properties, client }
    }

    fn build_client(config: &RedisClientProperties) -> Client {
        let url = crate::service::gen_url(config, true);

        let client = Client::open(url).unwrap();

        client
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn get_client_mut(&mut self) -> &mut Client {
        &mut self.client
    }

    pub async fn get_connection(&self) -> Result<MultiplexedConnection, RedisError> {
        self.client.get_multiplexed_tokio_connection().await
    }

    pub(crate) async fn expired_key_listen(
        &self,
        mut service: Box<dyn RedisExpiredKeysEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut sink, mut stream) = self.get_client().get_async_pubsub().await?.split();
        sink.psubscribe(format!(
            "__keyevent@{}__:expired",
            self.properties.database().unwrap_or(0)
        ))
        .await?;

        ::tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                if let Ok(key) = msg.get_pattern() {
                    if let Value::BulkString(pattern) = key {
                        service.on_message(msg.get_payload_bytes(), &pattern).await;
                    }
                }
            }
        });

        Ok(())
    }

    pub fn properties(&self) -> &RedisClientProperties {
        &self.properties
    }
}

impl Deref for RedisService {
    type Target = Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for RedisService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}
