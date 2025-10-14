use std::sync::Arc;

use async_trait::async_trait;
use axum::{http, response::IntoResponse};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    error::idempotency_error::IdempotencyError,
    traits::{store::idempotency_store::IdempotencyStore, to_error_response::ToErrorResponse},
};

/// 内存中的存储条目
#[derive(Debug, Clone)]
struct MemoryStorageEntry<T> {
    value: T,
    expires_at: Option<u64>,
}

#[derive(Clone)]
pub struct MemoryIdempotencyStore<T> {
    storage: Arc<RwLock<HashMap<Box<str>, MemoryStorageEntry<T>>>>,
}

impl<T> MemoryIdempotencyStore<T>
where
    T: Clone + Send + Sync,
    T: 'static,
{
    pub fn new() -> Self {
        let store = Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        };

        Self::start_backend_task(store.to_owned());

        store
    }

    fn start_backend_task(store: Self) {
        let store = store;
        tokio::spawn(async move {
            let next_time = 7000;
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(next_time));

            loop {
                store.storage.write().await.retain(|_, entry| {
                    entry
                        .expires_at
                        .map_or(true, |expires| expires > Self::current_timestamp())
                });

                interval.tick().await;
            }
        });
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[async_trait]
impl<T> IdempotencyStore for MemoryIdempotencyStore<T>
where
    T: Send + Sync + Clone + for<'de> Deserialize<'de> + Serialize + 'static,
{
    type Value = T;

    async fn check_and_store(
        &self,
        key: &str,
        value: Option<Self::Value>,
        ttl: Option<u64>,
    ) -> Result<Option<Self::Value>, IdempotencyError> {
        let mut storage = self.storage.write().await;
        let now = Self::current_timestamp();

        // 清理过期的条目
        storage.retain(|_, entry| entry.expires_at.map_or(true, |expires| expires > now));

        if let Some(entry) = storage.get(key) {
            // Key 存在，返回之前的值
            Ok(Some(entry.value.clone()))
        } else if let Some(value) = value {
            // Key 不存在，存储新值
            let expires_at = ttl.map(|ttl| now + ttl);
            let entry = MemoryStorageEntry {
                value: value.clone(),
                expires_at,
            };
            storage.insert(key.into(), entry);
            Ok(None)
        } else {
            // Key 不存在，但也没有提供要存储的值
            Ok(None)
        }
    }

    async fn delete(&self, key: &str) -> Result<(), IdempotencyError> {
        let mut storage = self.storage.write().await;
        storage.remove(key);
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<usize, IdempotencyError> {
        let mut storage = self.storage.write().await;
        let now = Self::current_timestamp();
        let before_len = storage.len();

        storage.retain(|_, entry| entry.expires_at.map_or(true, |expires| expires > now));

        Ok(before_len - storage.len())
    }

    async fn exists(&self, key: &str) -> Result<bool, IdempotencyError> {
        let storage = self.storage.read().await;
        let now = Self::current_timestamp();

        Ok(storage
            .get(key)
            .map(|entry| entry.expires_at.map_or(true, |expires| expires > now))
            .unwrap_or(false))
    }
}

impl<T> ToErrorResponse for MemoryIdempotencyStore<T>
where
    T: Send + Sync,
{
    fn to_error_response(&self, error_message: Option<String>) -> axum::response::Response {
        (
            http::StatusCode::BAD_REQUEST,
            if let Some(message) = error_message {
                message
            } else {
                "Bad request".to_string()
            },
        )
            .into_response()
    }
}
