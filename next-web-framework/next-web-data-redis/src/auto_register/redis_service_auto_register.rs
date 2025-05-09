use std::sync::Arc;

use next_web_core::{
    ApplicationContext, AutoRegister, async_trait, context::properties::ApplicationProperties,
    core::service::Service,
};
use redis::{Cmd, ConnectionLike};
use rudi_dev::Singleton;

use crate::{
    core::event::expired_keys_event::RedisExpiredKeysEvent,
    properties::redis_properties::RedisClientProperties, service::redis_service::RedisService,
};

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
            let redis_lock_service = crate::service::redis_lock_service::RedisLockService::new(vec![url]);
            let service_name = redis_lock_service.service_name();
            ctx.insert_singleton_with_name(
                redis_lock_service,
                service_name
            );
        }

        // Listen for expired keys
        #[cfg(feature = "expired-key-listener")]
        {
            let services = ctx.resolve_by_type::<Box<dyn RedisExpiredKeysEvent>>();
            if let Some(service) = services.first() {
                redis_service.expired_key_listen(service.to_owned()).await?;
            }
        }

        let service_name = redis_service.service_name();
        ctx.insert_singleton_with_name(redis_service, service_name);

        Ok(())
    }
}
