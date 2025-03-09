use crate::autoregister::auto_register::AutoRegister;
use crate::middleware::check_status::MiddlewareCheckStatus;
use crate::{
    autoconfigure::context::redis_properties::RedisProperties, manager::redis_manager::RedisManager,
};

#[cfg(feature = "redis_lock")]
use crate::middleware::redis::redis_lock::RedisLockManager;

use deadpool_redis::{Config, Runtime};

pub struct RedisAutoregister(pub RedisProperties);

impl AutoRegister for RedisAutoregister {

    fn name(&self) -> &'static str {
        "RedisAutoRegister"
    }

    
    fn register(&self, ctx: &mut rudi::Context) -> Result<(), Box<dyn std::error::Error>> {
        let password = self.0.password();
        let host = self.0.host();
        let port = self.0.port();

        // connect to redis
        let url = format!(
            "redis://{}{}:{}{}",
            password.map(|s| format!(":{}@", s)).unwrap_or_default(),
            if host.is_empty() { "localhost" } else { host },
            port,
            format!("/{}", self.0.database())
        );
        let cfg = Config::from_url(url.clone());

        let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();

        // add redis pool to context
        let redis_manager = RedisManager::new(pool, url.clone());

        // check  status
        futures::executor::block_on(redis_manager.status())?;

        ctx.insert_singleton_with_name::<RedisManager, String>(redis_manager, String::from("redisManager"));
        println!("RedisAutoregister registered successfully!");

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
        Ok(())
    }
}
