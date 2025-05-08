use std::sync::Arc;

use next_web_core::{
    async_trait, context::properties::ApplicationProperties, core::service::{self, Service}, ApplicationContext, AutoRegister
};
use redis::{Cmd, Commands, ConnectionLike};
use rudi_dev::Singleton;
use tracing::debug;

use crate::{
    properties::redis_properties::RedisClientProperties,
    service::redis_service::RedisService,
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

        // register redis_lock manager to context
        #[cfg(feature = "redis_lock")]
        {
            let redis_lock_manager = RedisLockManager::new(vec![url]);
            ctx.insert_singleton_with_name::<RedisLockManager, String>(
                redis_lock_manager,
                String::from("redisLockManager"),
            );
            println!("RedisLockAutoregister registered successfully!");
        }

        let service_name = redis_service.service_name();
        ctx.insert_singleton_with_name(redis_service, service_name);

        Ok(())
    }
}
