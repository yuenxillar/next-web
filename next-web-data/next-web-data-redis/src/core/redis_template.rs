use std::{error::Error, ops::Deref, sync::Arc, time::Duration};

use next_web_core::impl_service;
use redis::{AsyncConnectionConfig, Client, RedisResult, aio::MultiplexedConnection};

use crate::autoconfigure::redis_properties::RedisProperties;

/// Default Redis service exposed by the starter.
///
/// It wraps a `redis::Client` together with a small pool of multiplexed
/// connections so application code can resolve and use Redis immediately.
#[derive(Clone)]
pub struct RedisTemplate {
    /// Underlying Redis client.
    client: Arc<Client>,
    /// Redis async connection configuration.
    connection_config: Option<Arc<AsyncConnectionConfig>>,
}

impl RedisTemplate {
    /// Create a new Redis template from a Redis URL.
    pub fn with_url<U>(url: U) -> Result<Self, Box<dyn Error>>
    where
        U: AsRef<str>,
    {
        let client = Arc::new(Client::open(url.as_ref())?);
        let connection_config = Default::default();

        Ok(Self {
            client,
            connection_config,
        })
    }

    /// Create a new Redis service from  RedisProperties.
    pub fn with_properties(properties: &RedisProperties) -> Result<Self, Box<dyn Error>> {
        let mut redis_template = Self::with_url(properties.to_url())?;

        let connection_config =
            if properties.connect_timeout().is_some() || properties.response_timeout().is_some() {
                let config = AsyncConnectionConfig::default()
                    .set_response_timeout(
                        properties
                            .response_timeout()
                            .map(|time| Duration::from_millis(time)),
                    )
                    .set_connection_timeout(
                        properties
                            .connect_timeout()
                            .map(|time| Duration::from_millis(time)),
                    );

                Arc::new(config).into()
            } else {
                Default::default()
            };

        redis_template.connection_config = connection_config;

        Ok(redis_template)
    }

    /// Return the underlying Redis client.
    pub fn client(&self) -> &Arc<Client> {
        &self.client
    }

    /// Return the next multiplexed connection using round-robin selection.
    pub async fn get_connection(&self) -> RedisResult<MultiplexedConnection> {
        if let Some(config) = self.connection_config.as_ref() {
            self.client
                .get_multiplexed_async_connection_with_config(config)
                .await
        } else {
            self.client.get_multiplexed_async_connection().await
        }
    }
}

impl Deref for RedisTemplate {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl_service!(RedisTemplate);
