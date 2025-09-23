use std::any::Any;
use std::sync::RwLock;

use hashbrown::HashMap;
use std::borrow::Cow;
use std::sync::Arc;

use crate::models::any_value::AnyValue;

/// Support AnyMap caching for different data types
///
/// 支持不同数据类型的 AnyMap 缓存
#[derive(Clone)]
pub struct AnyMap {
    /// The underlying concurrent hash map storing cache items
    ///
    /// 存储缓存项的底层并发哈希表
    data: Arc<RwLock<HashMap<Cow<'static, str>, AnyValue>>>,
}

impl AnyMap {
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
    pub fn set<K, V>(&self, key: K, value: V)
    where
        K: Into<Cow<'static, str>>,
        V: Into<AnyValue>,
    {
        self.data
            .write()
            .map(|mut s| s.insert(key.into(), value.into()))
            .ok();
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
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<AnyValue> {
        self.data
            .read()
            .map(|s| s.get(key.as_ref()).map(|s| s.to_owned()))
            .unwrap_or_default()
    }

    /// 删除键
    pub fn remove<K: AsRef<str>>(&self, key: K) -> Option<AnyValue> {
        self.data
            .write()
            .map(|mut s| s.remove(key.as_ref()))
            .unwrap_or_default()
    }

    /// 检查键是否存在
    pub fn exists<K: AsRef<str>>(&self, key: K) -> bool {
        self.data
            .read()
            .map(|s| s.contains_key(key.as_ref()))
            .unwrap_or_default()
    }

    /// 清除所有键
    pub fn clear(&mut self) {
        let _ = self.data.write().map(|mut s| s.clear());
    }

    /// 获取缓存项数量
    pub fn len(&self) -> usize {
        self.data.read().map(|s| s.len()).unwrap_or_default()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.data.read().map(|s| s.is_empty()).unwrap_or_default()
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
