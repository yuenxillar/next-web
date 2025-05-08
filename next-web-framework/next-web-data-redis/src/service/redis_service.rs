use std::ops::{Deref, DerefMut};

use futures::StreamExt;
use next_web_core::core::service::Service;
use redis::{aio::tokio, Client, Value};

use crate::properties::redis_properties::RedisClientProperties;

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
        let password = config.password();
        let host = config.host().unwrap_or("localhost".into());
        let port = config.port().unwrap_or(6379);

        // connect to redis
        let url = format!(
            "redis://{}{}:{}{}?protocol=resp3",
            password.map(|s| format!(":{}@", s)).unwrap_or_default(),
            host,
            port,
            format!("/{}", config.database().unwrap_or(0))
        );

        let client = Client::open(url).unwrap();

        client
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn get_client_mut(&mut self) -> &mut Client {
        &mut self.client
    }

    pub(crate) async fn expired_key_listen(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (mut sink, mut stream) = self.get_client().get_async_pubsub().await?.split();
        sink.psubscribe(format!(
            "__keyevent@{}__:expired",
            self.properties.database().unwrap_or(0)
        ))
        .await?;

        ::tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                if let Ok(key) = msg.get_pattern() {
                    if let Value::BulkString(data) = key {
                        let key = String::from_utf8(data.to_vec()).unwrap();
                        println!("key expired: {}", key);
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
