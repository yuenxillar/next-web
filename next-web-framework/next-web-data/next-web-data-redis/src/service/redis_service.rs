use std::{
    ops::{Deref, DerefMut},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use next_web_core::traits::{service::Service, singleton::Singleton};
use redis::{Client, aio::MultiplexedConnection};

#[cfg(feature = "expired-key-listener")]
use crate::core::event::expired_keys_event::RedisExpiredKeysEvent;
use crate::properties::redis_properties::RedisClientProperties;

/// Redis服务结构体
/// Redis service
#[derive(Clone)]
pub struct RedisService {
    /// Redis客户端配置属性
    /// 
    /// Redis client configuration properties
    properties: RedisClientProperties,
    /// Redis客户端实例
    /// 
    ///  Redis client instance
    client: Client,
    /// Redis连接池
    /// 
    /// Redis connection pool
    pub(crate) connections: Vec<MultiplexedConnection>,
    /// 当前连接索引
    /// 
    /// Current connection index
    index: Arc<AtomicUsize>,
}


impl Singleton  for RedisService {}
impl Service    for RedisService {}


impl RedisService {
    /// 创建新的Redis服务实例
    ///  Create new Redis service instance
    pub fn new(properties: RedisClientProperties) -> Self {
        let client = Self::build_client(&properties);
        
        let connections = Vec::with_capacity(7);
        let index = Arc::new(AtomicUsize::new(0));
        Self {
            properties,
            client,
            connections,
            index,
        }
    }

    /// 构建Redis客户端
    /// Build Redis client
    fn build_client(config: &RedisClientProperties) -> Client {
        let url = crate::service::gen_url(config, true);
        let client = Client::open(url).unwrap();
        client
    }

    /// 获取Redis客户端引用
    ///  Get Redis client reference
    pub fn get_client(&self) -> &Client {
        &self.client
    }

    /// 获取当前Redis连接
    ///  Get current Redis connection
    pub fn get_connection(&self) -> Option<MultiplexedConnection> {
        if self.connections.is_empty() {
            return None;
        }

        let idx = self.index.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |i| Some((i + 1) % self.connections.len())
        ).ok()?;

        self.connections.get(idx).cloned()
    }

    /// 过期键监听器
    ///  Expired key listener
    #[cfg(feature = "expired-key-listener")]
    pub(crate) async fn expired_key_listener(
        &self,
        mut service: Box<dyn RedisExpiredKeysEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut sink, mut stream) = self.get_client().get_async_pubsub().await?.split();
        sink.psubscribe(format!(
            "__keyevent@{}__:expired",
            self.properties.database().unwrap_or(0)
        ))
        .await?;
        ::tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                if let Ok(key) = msg.get_pattern() {
                    if let Value::BulkString(pattern) = key {
                        service.on_message(msg.get_payload_bytes(), &pattern).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// 获取配置属性
    ///  Get configuration properties
    pub fn properties(&self) -> &RedisClientProperties {
        &self.properties
    }

    /// 获取当前连接索引
    /// Get current connection index
    pub fn index(&self) -> usize {
        self.index.load(Ordering::Relaxed)
    }
}

impl Deref for RedisService {
    type Target = Client;
    /// 解引用为Redis客户端
    ///  Dereference to Redis client
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for RedisService {
    /// 可变解引用为Redis客户端
    ///  Mutable dereference to Redis client
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}
