use std::any::Any;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use tokio::sync::RwLock;

use std::sync::Arc;

use crate::anys::any_value::AnyValue;

/// Support AnyMap caching for different data types
///
/// 支持不同数据类型的 AnyMap 缓存
#[derive(Clone, Default)]
pub struct AnyMap<K = String, V = AnyValue> {
    /// The underlying concurrent hash map storing cache items
    ///
    /// 存储缓存项的底层并发哈希表
    data: Arc<RwLock<HashMap<K, V>>>,
}

impl<K, V> AnyMap<K, V>
where
    K: Hash + Eq,
    K: Clone,
    V: Clone,
{
    /// Creates a new AnyMap instance
    /// 创建一个新的 AnyMap 实例
    ///
    /// # Examples
    /// 示例
    ///
    /// ```
    /// let cache = AnyMap::new();
    /// ```
    pub fn new() -> Self {
        let map = Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        };
        map
    }

    /// Sets a key-value pair with optional time-to-live
    /// 设置键值对，带有可选的存活时间
    ///
    /// # Arguments
    /// 参数
    ///
    /// * `key` - Cache key
    /// * `key` - 缓存键
    /// * `value` - Value to cache
    /// * `value` - 要缓存的值
    /// * `ttl` - Optional time-to-live duration
    /// * `ttl` - 可选的存活时间
    ///
    /// # Examples
    /// 示例
    ///
    /// ```
    /// cache.set("my_key", "my_value", Some(Duration::from_secs(60)));
    /// ```
    pub async fn set(&self, key: K, value: V)
    {
        self.data.write().await.insert(key, value);
    }

    /// Gets a value by key, returns None if expired or not found
    /// 通过键获取值，如果过期或不存在则返回 None
    ///
    /// # Arguments
    /// 参数
    ///
    /// * `key` - Key to look up
    /// * `key` - 要查找的键
    ///
    /// # Examples
    /// 示例
    ///
    /// ```
    /// let value = cache.get("my_key");
    /// ```
    pub async fn get<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.read().await.get(key).map(|s| s.clone())
    }

    /// 删除键
    pub async fn remove<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.write().await.remove(key)
    }

    /// 检查键是否存在
    pub async fn exists<Q: ?Sized>(&self, key: &Q) -> bool 
      where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.read().await.contains_key(key)
    }

    /// 清除所有键
    pub async fn clear(&self) {
        let _ = self.data.write().await.clear();
    }

    /// 获取缓存项数量
    pub async fn len(&self) -> usize {
        self.data.read().await.len()
    }

    /// 检查缓存是否为空
    pub async fn is_empty(&self) -> bool {
        self.data.read().await.is_empty()
    }

    pub async fn extend(&self, other: Self) {
        self.data
            .write()
            .await
            .extend(other.data.read().await.clone())
    }

    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&K, &V),
    {
        self.data
            .try_read()
            .map(|map| map.iter().for_each(|(k, v)| f(k, v)))
            .ok();
    }

    pub async fn filter<F>(&self, f: F) -> HashMap<K, V>
    where
        F: Fn(&K, &V) -> bool,
    {
        let filtered_map: HashMap<K, V> = self
            .data
            .read()
            .await
            .iter()
            .filter(|(k, v)| f(k, v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        filtered_map
    }
}

/// 缓存对象包装器
/// Cache object wrapper
pub struct CacheObject<T>(pub T)
where
    T: Any + Clone + Send + Sync;

impl<T: 'static + Any + Clone + Send + Sync> Into<AnyValue> for CacheObject<T> {
    fn into(self) -> AnyValue {
        AnyValue::Object(Box::new(self.0))
    }
}
