use std::str;
use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use next_web::application::Application;
use next_web::extract::find_singleton::FindSingleton;
use next_web::macros::bind::singleton;
use next_web_core::context::properties::ApplicationProperties;
use next_web_core::{ApplicationContext, async_trait};
use next_web_data_redis::AsyncCommands;
use next_web_data_redis::connection::default_message::DefaultMessage;
use next_web_data_redis::connection::message::Message;
use next_web_data_redis::core::redis_template::RedisTemplate;
use next_web_data_redis::listener::key_expiration_event_message_listener::KeyExpirationEventMessageListener;

#[singleton(binds = [Self::into_expired_key_listener])]
#[derive(Clone)]
pub(crate) struct TestExpiredKeyListener {
    #[autowired(name = "redisTemplate")]
    pub redis_template: RedisTemplate,
}

impl TestExpiredKeyListener {
    fn into_expired_key_listener(self) -> Arc<dyn KeyExpirationEventMessageListener> {
        Arc::new(self)
    }
}

#[async_trait]
impl KeyExpirationEventMessageListener for TestExpiredKeyListener {
    async fn on_message(&self, message: &DefaultMessage, pattern: &[u8]) {
        println!(
            "Expired key: {}, pattern: {}",
            String::from_utf8_lossy(message.body()),
            String::from_utf8_lossy(pattern)
        );

        if let Some(mut con) = self
            .redis_template
            .get_multiplexed_async_connection()
            .await
            .ok()
        {
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
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/get/{key}", axum::routing::post(get_cache))
            .route("/set", axum::routing::post(set_cache))
    }
}

async fn get_cache(
    FindSingleton(redis_template): FindSingleton<RedisTemplate>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    if let Some(mut con) = redis_template.get_multiplexed_async_connection().await.ok() {
        return match con.get(&key).await {
            Ok(value) => return value,
            Err(e) => format!("Get Error: {}", e.to_string()),
        };
    }
    "No value found".to_string()
}

async fn set_cache(
    FindSingleton(redis_template): FindSingleton<RedisTemplate>,
    Query(cache): Query<KeyValue>,
) -> impl IntoResponse {
    if let Some(mut con) = redis_template.get_multiplexed_async_connection().await.ok() {
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
