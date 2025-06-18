use std::{sync::Arc, time::Duration};

use next_web_core::{
    ApplicationContext, AutoRegister, async_trait, context::properties::ApplicationProperties,
    core::service::Service,
};
use redis::{Cmd, ConnectionLike};
use rudi_dev::Singleton;

use crate::{
    properties::redis_properties::RedisClientProperties, service::redis_service::RedisService,
};

#[cfg(feature = "expired-key-listener")]
use crate::core::event::expired_keys_event::RedisExpiredKeysEvent;

/// Register the `DatabaseService` as a singleton with the `DatabaseServiceAutoRegister` type.
#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct RedisServiceAutoRegister(pub RedisClientProperties);

impl RedisServiceAutoRegister {
    /// Convert the current structure into a dynamically dispatched `AutoRegister` type
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for RedisServiceAutoRegister {
    /// Return the singleton name to identify the service
    fn singleton_name(&self) -> &'static str {
        ""
    }

    /// Asynchronous registration method
    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Clone theconfiguration properties
        let client_properties = self.0.clone();

        let mut redis_service = RedisService::new(client_properties);

        redis_service.req_command(Cmd::new().arg("PING"))?;

        // Register redis_lock manager to context
        #[cfg(feature = "lock")]
        {
            let url = crate::service::gen_url(&self.0, false);
            let redis_lock_service =
                crate::service::redis_lock_service::RedisLockService::new(vec![url]);
            let service_name = redis_lock_service.service_name();
            ctx.insert_singleton_with_name(redis_lock_service, service_name);
        }

        let service_name = redis_service.service_name();
        for _ in 0..7 {
            let connect = redis_service
                .get_multiplexed_tokio_connection_with_response_timeouts(
                    Duration::from_secs(5),
                    Duration::from_secs(5),
                )
                .await
                .unwrap();
            redis_service.connections.push(connect);
        }

        ctx.insert_singleton_with_name(redis_service, service_name.to_owned());

        // Listen for expired keys
        #[cfg(feature = "expired-key-listener")]
        {
            let services = ctx.resolve_by_type::<Box<dyn RedisExpiredKeysEvent>>();
            if let Some(service) = services.first() {
                let rs = ctx.get_single_with_name::<RedisService>(service_name);
                rs.expired_key_listener(service.to_owned()).await?;
            }
        }

        Ok(())
    }
}
