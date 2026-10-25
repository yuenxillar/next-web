use next_web_context::ApplicationContextExt;
use next_web_core::Ordered;
use next_web_core::util::StringUtils;
use std::{borrow::Cow, error::Error, sync::Arc};

use next_web_core::{
    ApplicationContext, async_trait, config::auto_configuration::AutoConfiguration,
};
use next_web_macros::singleton;
use redis::{Cmd, ConnectionLike, TypedCommands};

use crate::{
    autoconfigure::redis_properties::RedisProperties,
    core::listener::{
        key_expiration_event_message_listener::{
            KEYEVENT_EXPIRED_TOPIC, KeyExpirationEventMessageListener,
        },
        redis_message_listener_container::RedisMessageListenerContainer,
    },
    core::redis_template::RedisTemplate,
};

/// Auto-configuration for Data's Redis support.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct RedisAutoConfiguration {
    redis_properties: RedisProperties,
}

impl RedisAutoConfiguration {
    fn redis_template_auto_configuration(
        &self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn Error>> {
        let redis_template = match ctx.get_singleton_option_with_default_name::<RedisTemplate>() {
            Some(redis_template) => Cow::Borrowed(redis_template),
            None => {
                let redis_template = match self
                    .redis_properties
                    .url()
                    .filter(|s| StringUtils::has_text(*s))
                {
                    Some(url) => RedisTemplate::with_url(url)?,
                    None => RedisTemplate::with_properties(&self.redis_properties)?,
                };

                Cow::Owned(redis_template)
            }
        };

        // Verify if Redis connection is available, use ping.
        if let Err(err) = redis_template
            .get_connection_with_timeout(std::time::Duration::from_millis(2000))?
            .ping()
        {
            return Err(format!("Redis ping failed: {err}").into());
        }

        if let Cow::Owned(redis_template) = redis_template {
            ctx.insert_singleton_with_default_name(redis_template)
        };

        Ok(())
    }

    fn redis_message_listener_container_auto_configuration(
        &self,
        ctx: &mut dyn ApplicationContext,
    ) {
        if ctx.contains_singleton_with_default_name::<RedisMessageListenerContainer>() {
            return;
        }

        let client = ctx
            .get_singleton_option_with_default_name::<RedisTemplate>()
            .map(|temp| temp.client())
            .map(ToOwned::to_owned)
            .expect("RedisTemplate singleton is not registered in context;");
        ctx.insert_singleton_with_default_name(RedisMessageListenerContainer::new(client));
    }

    async fn redis_key_expiration_event_message_listener_auto_configuration(
        &self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn Error>> {
        // Register listener to Redis message listener container.
        let message_listeners = ctx.resolve_by_type::<Arc<dyn KeyExpirationEventMessageListener>>();
        if message_listeners.is_empty() {
            return Ok(());
        }

        let redis_template =  ctx.get_singleton_option_with_default_name::<RedisTemplate>()
            .expect("RedisTemplate singleton is not registered in context; Redis key-expiry notification cannot be enabled");
        // Enable the Redis server-side notification switch required by expired-key listeners.
        redis_template
            .get_connection_with_timeout(std::time::Duration::from_millis(1500))?
            .req_command(Cmd::new().arg(&["CONFIG", "SET", "notify-keyspace-events", "Ex"]))?;

        let container = ctx
            .get_singleton_option_with_default_name::<RedisMessageListenerContainer>()
            .expect("RedisMessageListenerContainer singleton is not registered in context;");
        for message_listener in message_listeners {
            container
                .add_message_listener(message_listener, KEYEVENT_EXPIRED_TOPIC)
                .await?;
        }

        Ok(())
    }

    async fn redis_lock_auto_configuration(
        &self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn Error>> {
        // #[cfg(feature = "distributed-lock")]
        // if self.redis_properties.lock_enabled() {
        //     let config =
        //         self.redis_properties
        //             .lock_config()
        //             .map_err(|error| -> Box<dyn Error> {
        //                 std::io::Error::new(std::io::ErrorKind::InvalidInput, error).into()
        //             })?;
        //     let lock_service =
        //         crate::service::lock::RedisDistributedLockService::with_direct_client(
        //             redis_template.client().clone(),
        //             config,
        //         )?;
        //     let service_name = lock_service.singleton_name();
        //     ctx.insert_singleton_with_name(lock_service, service_name);
        // }

        Ok(())
    }
}

#[async_trait]
impl AutoConfiguration for RedisAutoConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        self.redis_template_auto_configuration(ctx)?;
        self.redis_message_listener_container_auto_configuration(ctx);
        self.redis_key_expiration_event_message_listener_auto_configuration(ctx)
            .await?;
        self.redis_lock_auto_configuration(ctx).await?;

        Ok(())
    }
}

impl RedisAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

impl Ordered for RedisAutoConfiguration {}
