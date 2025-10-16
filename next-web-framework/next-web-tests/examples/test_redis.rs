use std::str;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use next_web_core::context::properties::ApplicationProperties;
use next_web_core::{async_trait, ApplicationContext};
use next_web_data_redis::core::event::expired_keys_event::RedisExpiredKeysEvent;
use next_web_data_redis::service::redis_service::RedisService;
use next_web_data_redis::AsyncCommands;
use next_web_dev::application::Application;
use next_web_dev::extract::find_singleton::FindSingleton;
use next_web_dev::Singleton;

#[Singleton(binds = [Self::into_expired_key_listener])]
#[derive(Clone)]
pub(crate) struct TestExpiredKeyListener {
    #[resource(name = "redisService")]
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
            let new_value: i64 = con.incr("keyabc", 1).await.unwrap();
            println!("new value: {}", new_value);
        }
    }
}

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/get/{key}", axum::routing::post(get_cache))
            .route("/set", axum::routing::post(set_cache))
    }
}

async fn get_cache(
    FindSingleton(redis_service): FindSingleton<RedisService>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    if let Some(mut con) = redis_service.get_connection() {
        return match con.get(&key).await {
            Ok(value) => return value,
            Err(e) => format!("Get Error: {}", e.to_string()),
        };
    }
    "No value found".to_string()
}

async fn set_cache(
    FindSingleton(redis_service): FindSingleton<RedisService>,
    Query(cache): Query<KeyValue>,
) -> impl IntoResponse {
    if let Some(mut con) = redis_service.get_connection() {
        match con.set_ex::<_, _, ()>(cache.key, cache.value, 5).await {
            Ok(_) => {}
            Err(e) => return format!("Set Error: {}", e.to_string()),
        };
    }
    "Ok".to_string()
}

#[derive(serde::Deserialize)]
struct KeyValue {
    pub key: String,
    pub value: String,
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
