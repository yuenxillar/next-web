use deadpool_redis::redis::{AsyncConnectionConfig, Client, PushKind, Value::BulkString};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::event::redis_expired_event::RedisExpiredEvent;
#[derive(Clone)]
pub struct RedisManager {
    pool: deadpool_redis::Pool,
    url: String,
}

impl RedisManager {
    pub fn new(pool: deadpool_redis::Pool, url: String) -> Self {
        Self { pool, url }
    }

    pub async fn get_conn(&self) -> Result<deadpool_redis::Connection, deadpool_redis::PoolError> {
        self.pool.get().await
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub async fn expired_event(
        &self,
        handle: Arc<Mutex<dyn RedisExpiredEvent>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::open(format!("{}/?protocol=resp3", self.url))?;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let config = AsyncConnectionConfig::new().set_push_sender(tx);
        let mut con = client
            .get_multiplexed_async_connection_with_config(&config)
            .await?;
        let channel_pattern = format!(
            "__keyevent@{}__:expired",
            self.url
                .split("/")
                .collect::<Vec<&str>>()
                .last()
                .unwrap_or(&"*")
        );
        con.psubscribe(&channel_pattern).await?;
        info!("Subscribed to {} successfully!", channel_pattern);

        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                println!("Redis expired message:{:?}", msg);
                match msg.kind {
                    PushKind::PMessage => {
                        // handle expired event
                        let data = msg.data;
                        if !data.is_empty() {
                            // key name
                            if let Some(msg) = data.get(2) {
                                match msg.to_owned() {
                                    BulkString(key) => {
                                        let pattern = data
                                            .get(1)
                                            .map(|msg| {
                                                if let BulkString(msg) = msg.to_owned() {
                                                    return String::from_utf8(msg)
                                                        .unwrap_or_default();
                                                }
                                                String::default()
                                            })
                                            .unwrap_or_default();
                                        let key = String::from_utf8(key).unwrap_or_default();
                                        handle.lock().await.on_message(key, pattern).await;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {
                        // println!("Unknown Redis message:{:?}", msg);
                    }
                }
            }
        });
        Ok(())
    }
}

use crate::middleware::check_status::MiddlewareCheckStatus;
use async_trait::async_trait;

#[async_trait]
impl MiddlewareCheckStatus for RedisManager {
    async fn status(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.pool.get().await?;
        Ok(())
    }
}
