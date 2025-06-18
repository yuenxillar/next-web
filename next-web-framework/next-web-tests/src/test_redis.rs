use next_web_core::async_trait;
use next_web_data_redis::core::event::expired_keys_event::RedisExpiredKeysEvent;
use next_web_data_redis::service::redis_service::RedisService;
use next_web_data_redis::AsyncCommands;
use next_web_dev::Singleton;

#[Singleton(binds = [Self::into_expired_key_listener])]
#[derive(Clone)]
pub(crate) struct TestExpiredKeyListener {
    #[autowired(name = "redisService")]
    pub redis_service: RedisService,
}

impl TestExpiredKeyListener {
    fn into_expired_key_listener(self) -> Box<dyn RedisExpiredKeysEvent> {
        Box::new(self)
    }
}

#[async_trait]
impl RedisExpiredKeysEvent for TestExpiredKeyListener {
    async fn on_message(&mut self, message: &[u8], pattern: &[u8]) {
        println!(
            "Expired key: {}, pattern: {}",
            String::from_utf8_lossy(message),
            String::from_utf8_lossy(pattern)
        );

        if let Some(mut con) = self.redis_service.get_connection() {
            let new_value: i64  = con.incr("keyabc", 1).await.unwrap();
            println!("new value: {}", new_value);
        }
    }
}