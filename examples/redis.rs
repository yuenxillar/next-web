use std::str;
use std::sync::Arc;

use axum::extract::{Path, Query};
use next_web::extract::find_singleton::FindSingleton;
use next_web::macros::bind::{post_mapping, singleton};
use next_web::{Application, NextWebApplication};
use next_web_core::async_trait;
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

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[post_mapping(path = "/get/{key}")]
async fn get_cache(
    FindSingleton(redis_template): FindSingleton<RedisTemplate>,
    Path(key): Path<String>,
) -> String {
    match redis_template.get_multiplexed_async_connection().await {
        Ok(mut con) => match con.get::<_, String>(&key).await {
            Ok(value) => value,
            Err(error) => format!("Get Error: {error}"),
        },
        Err(_) => "No value found".to_string(),
    }
}

#[post_mapping(path = "/set")]
async fn set_cache(
    FindSingleton(redis_template): FindSingleton<RedisTemplate>,
    Query(cache): Query<KeyValue>,
) -> String {
    match redis_template.get_multiplexed_async_connection().await {
        Ok(mut con) => match con.set_ex::<_, _, ()>(cache.key, cache.value, 5).await {
            Ok(()) => "Ok".to_string(),
            Err(error) => format!("Set Error: {error}"),
        },
        Err(_) => "Ok".to_string(),
    }
}

#[derive(serde::Deserialize)]
struct KeyValue {
    pub key: String,
    pub value: String,
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
