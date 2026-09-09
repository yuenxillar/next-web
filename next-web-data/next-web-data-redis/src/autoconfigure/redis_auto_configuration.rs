use std::{error::Error, sync::Arc};

#[cfg(feature = "distributed-lock")]
use next_web_core::traits::singleton::Singleton;
use next_web_core::{
    ApplicationContext, async_trait, traits::config::auto_configuration::AutoConfiguration,
};
use next_web_macros::auto_configuration;
use redis::{Cmd, ConnectionLike, TypedCommands};
use rudi_dev::singleton;

use crate::{
    autoconfigure::redis_properties::RedisProperties,
    core::redis_template::RedisTemplate,
    listener::{
        key_expiration_event_message_listener::{
            KEYEVENT_EXPIRED_TOPIC, KeyExpirationEventMessageListener,
        },
        redis_message_listener_container::RedisMessageListenerContainer,
    },
};

/// Auto Configuration entry for the Redis starter.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct RedisAutoConfiguration {
    redis_properties: RedisProperties,
}

#[async_trait]
impl AutoConfiguration for RedisAutoConfiguration {
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>> {
        let redis_template = RedisTemplate::with_properties(&self.redis_properties)?;

        // Validate the Redis connection.
        if !redis_template
            .get_connection_with_timeout(std::time::Duration::from_millis(2000))?
            .ping()
            .is_ok()
        {
            return Err("Redis connection failed".into());
        }

        ctx.insert_singleton_with_default_name(redis_template.to_owned());
        #[cfg(feature = "distributed-lock")]
        if self.redis_properties.lock_enabled() {
            let config =
                self.redis_properties
                    .lock_config()
                    .map_err(|error| -> Box<dyn Error> {
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, error).into()
                    })?;
            let lock_service =
                crate::service::lock::RedisDistributedLockService::with_direct_client(
                    redis_template.client().clone(),
                    config,
                )?;
            let service_name = lock_service.singleton_name();
            ctx.insert_singleton_with_name(lock_service, service_name);
        }

        // Register listener to Redis message listener container.
        let mut message_listeners =
            ctx.resolve_by_type::<Arc<dyn KeyExpirationEventMessageListener>>();

        let mut listener_container =
            RedisMessageListenerContainer::new(redis_template.client().clone());
        if let Some(message_listener) = message_listeners.pop() {
            listener_container
                .add_message_listener(message_listener, KEYEVENT_EXPIRED_TOPIC)
                .await?;

            // Enable the Redis server-side notification switch required by expired-key listeners.
            redis_template
                .get_connection_with_timeout(std::time::Duration::from_millis(1000))?
                .req_command(Cmd::new().arg(&["CONFIG", "SET", "notify-keyspace-events", "Ex"]))?;
        }

        ctx.insert_singleton_with_default_name(listener_container);

        Ok(())
    }
}

#[auto_configuration]
impl RedisAutoConfiguration {
    #[provider(conditional = [Self::redis_template_missing])]
    pub fn redis_template(redis_properties: &RedisProperties) -> RedisTemplate {
        RedisTemplate::with_properties(redis_properties)
        .expect(
            "Failed to create RedisTemplate. Please check Redis connection configuration and ensure Redis server is running",
        )
    }

    fn redis_template_missing(ctx: &ApplicationContext) -> bool {
        !ctx.contains_single::<RedisTemplate>()
    }
}

impl RedisAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}
