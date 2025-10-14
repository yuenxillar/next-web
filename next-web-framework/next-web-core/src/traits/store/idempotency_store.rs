use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    error::idempotency_error::IdempotencyError, traits::to_error_response::ToErrorResponse,
};

#[async_trait]
pub trait IdempotencyStore
where
    Self: Send + Sync,
    Self: ToErrorResponse,
{
    /// 存储值的类型
    type Value: Send + Sync + Clone + for<'de> Deserialize<'de> + Serialize;

    /// 检查并存储幂等性 key
    /// 如果 key 不存在，存储并返回 None
    /// 如果 key 存在，返回之前存储的结果
    async fn check_and_store(
        &self,
        key: &str,
        value: Option<Self::Value>,
        ttl: Option<u64>,
    ) -> Result<Option<Self::Value>, IdempotencyError>;

    /// 删除幂等性 key
    async fn delete(&self, key: &str) -> Result<(), IdempotencyError>;

    /// 清理过期的 key
    async fn cleanup_expired(&self) -> Result<usize, IdempotencyError>;

    /// 检查 key 是否存在（不返回值）
    async fn exists(&self, key: &str) -> Result<bool, IdempotencyError>;
}
