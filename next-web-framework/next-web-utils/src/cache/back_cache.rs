use dashmap::DashMap;
use regex::Regex;
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use super::cache_value::CacheValue;

/// A high-performance concurrent cache with expiration support
/// 支持过期时间的高性能并发缓存
#[derive(Clone)]
pub struct BackCache {
    /// The underlying concurrent hash map storing cache items
    /// 存储缓存项的底层并发哈希表
    data: Arc<DashMap<Cow<'static, str>, CacheItem>>,

    /// Handle to the background cleanup task
    /// 后台清理任务的句柄
    cleanup_task: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,

    /// Signal to control the cleanup task's execution
    /// 控制清理任务执行的信号量
    task_signal: Arc<AtomicBool>,
}

/// Internal representation of a cache item
/// 缓存项的内部表示
#[derive(Clone)]
struct CacheItem {
    /// The cached value
    /// 缓存的值
    pub value: CacheValue,

    /// Optional expiration time
    /// 可选的过期时间
    pub expires_at: Option<Instant>,
}

impl BackCache {
    /// Creates a new BackCache instance
    /// 创建一个新的 BackCache 实例
    ///
    /// # Examples
    /// 示例
    ///
    /// ```
    /// let cache = BackCache::new();
    /// ```
    pub fn new() -> Self {
        let cache = Self {
            data: Arc::new(DashMap::new()),
            cleanup_task: Arc::new(Mutex::new(None)),
            task_signal: Arc::new(AtomicBool::new(true)),
        };
        cache.start_cleanup_task();
        cache
    }

    /// Starts the background cleanup task
    /// 启动后台清理任务
    ///
    /// The task periodically removes expired items
    /// 该任务会定期清理过期的项目
    fn start_cleanup_task(&self) {
        let data = self.data.clone();
        let task_signal = self.task_signal.clone();

        let handle = thread::spawn(move || {
            while task_signal.load(Ordering::Relaxed) {
                let now = Instant::now();
                // Remove expired items
                // 移除过期项目
                data.retain(|_, item| item.expires_at.map_or(true, |expiry| expiry > now));

                // More precise timing control
                // 更精确的定时控制
                thread::sleep(
                    Duration::from_secs(1).min(
                        data.iter()
                            .filter_map(|item| item.expires_at)
                            .min()
                            .map(|expiry| expiry.saturating_duration_since(now))
                            .unwrap_or(Duration::from_secs(1)),
                    ),
                );
            }
        });

        if let Ok(mut task) = self.cleanup_task.lock() {
            *task = Some(handle);
        }
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
    pub fn set<K, V>(&self, key: K, value: V, ttl: Option<Duration>)
    where
        K: Into<Cow<'static, str>>,
        V: Into<CacheValue>,
    {
        let expires_at = ttl.map(|d| Instant::now() + d);
        self.data.insert(
            key.into(),
            CacheItem {
                value: value.into(),
                expires_at,
            },
        );
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
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<CacheValue> {
        let item = self.data.get(key.as_ref())?;
        if let Some(expiry) = item.expires_at {
            if expiry <= Instant::now() {
                self.data.remove(key.as_ref());
                return None;
            }
        }
        Some(item.value.clone())
    }

    /// Gets the remaining time-to-live for a key
    /// 获取键的剩余存活时间
    ///
    /// Returns None if the key doesn't exist or has no expiration
    /// 如果键不存在或没有设置过期时间，返回 None
    ///
    /// # Arguments
    /// 参数
    ///
    /// * `key` - Key to check
    /// * `key` - 要检查的键
    ///
    /// # Examples
    /// 示例
    ///
    /// ```
    /// let ttl = cache.ttl("my_key");
    /// ```
    pub fn ttl<K: AsRef<str>>(&self, key: K) -> Option<Duration> {
        let item = self.data.get(key.as_ref())?;
        item.expires_at
            .map(|expiry| expiry.saturating_duration_since(Instant::now()))
    }

    /// Finds keys containing the given pattern
    /// 查找包含给定模式的键
    ///
    /// # Arguments
    /// 参数
    ///
    /// * `pattern` - String pattern to search for
    /// * `pattern` - 要搜索的字符串模式
    ///
    /// # Examples
    /// 示例
    ///
    /// ```
    /// let results = cache.fuzzy_find("user");
    /// ```
    pub fn fuzzy_find(&self, pattern: &str) -> Vec<(String, CacheValue)> {
        self.data
            .iter()
            .filter(|item| item.key().contains(pattern))
            .map(|item| (item.key().to_string(), item.value.clone()))
            .collect()
    }

    /// Finds keys matching the given regular expression
    /// 查找匹配给定正则表达式的键
    ///
    /// # Arguments
    /// 参数
    ///
    /// * `regex` - Regular expression pattern
    /// * `regex` - 正则表达式模式
    ///
    /// # Examples
    /// 示例
    ///
    /// ```
    /// let results = cache.fuzzy_find_regex(r"user_\d+")?;
    /// ```
    pub fn fuzzy_find_regex(&self, regex: &str) -> Result<Vec<(String, CacheValue)>, regex::Error> {
        let re = Regex::new(regex)?;
        Ok(self
            .data
            .iter()
            .filter(|item| re.is_match(item.key()))
            .map(|item| (item.key().to_string(), item.value.clone()))
            .collect())
    }

    /// 删除键
    pub fn remove<K: AsRef<str>>(&self, key: K) -> Option<CacheValue> {
        self.data.remove(key.as_ref()).map(|(_, item)| item.value)
    }

    /// 检查键是否存在
    pub fn exists<K: AsRef<str>>(&self, key: K) -> bool {
        self.data.contains_key(key.as_ref())
    }

    /// 设置过期时间
    pub fn set_expire(&self, key: &str, ttl: Duration) -> bool {
        if let Some(mut item) = self.data.get_mut(key) {
            item.expires_at = Some(Instant::now() + ttl);
            true
        } else {
            false
        }
    }

    /// 清除所有键
    pub fn clear(&self) {
        self.data.clear();
    }

    /// 获取缓存项数量
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Drop for BackCache {
    /// Cleans up resources when the cache is dropped
    /// 当缓存被丢弃时清理资源
    ///
    /// Stops the background cleanup task
    /// 停止后台清理任务
    fn drop(&mut self) {
        self.task_signal.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {

    use crate::cache::cache_value::CacheObject;

    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_operations() {
        let cache = BackCache::new();

        // Test set and get
        cache.set("key1", "value1", None);
        let var = cache.get("key1").map(|s| s.as_string()).unwrap_or_default();

        assert_eq!(var, Some("value1".into()));

        // Test exists
        assert!(cache.exists("key1"));
        assert!(!cache.exists("nonexistent"));

        // Test remove
        assert_eq!(
            cache
                .remove("key1")
                .map(|s| s.as_string())
                .unwrap_or_default(),
            Some("value1".into())
        );
        assert!(!cache.exists("key1"));
    }

    #[test]
    fn test_expiration() {
        let cache = BackCache::new();

        // Set with short TTL
        cache.set("temp", "data", Some(Duration::from_millis(100)));
        assert!(cache.get("temp").is_some());

        // Wait for expiration
        thread::sleep(Duration::from_millis(150));
        assert!(cache.get("temp").is_none());
    }

    #[test]
    fn test_ttl() {
        let cache = BackCache::new();

        // Test with TTL
        cache.set("key", "value", Some(Duration::from_secs(1)));
        let ttl = cache.ttl("key").unwrap();
        assert!(ttl <= Duration::from_secs(1) && ttl > Duration::from_secs(0));

        // Test no TTL
        cache.set("permanent", "value", None);
        assert!(cache.ttl("permanent").is_none());

        // Test expired
        cache.set("expired", "value", Some(Duration::from_millis(1000)));
        thread::sleep(Duration::from_millis(1200));
        assert!(cache.ttl("expired").is_none());
    }

    #[test]
    fn test_set_expire() {
        let cache = BackCache::new();

        cache.set("key", "value", None);
        assert!(cache.ttl("key").is_none());

        assert!(cache.set_expire("key", Duration::from_secs(1)));
        assert!(cache.ttl("key").is_some());

        assert!(!cache.set_expire("nonexistent", Duration::from_secs(1)));
    }

    #[test]
    fn test_fuzzy_find() {
        let cache = BackCache::new();

        cache.set("user:1", "Alice", None);
        cache.set("user:2", "Bob", None);
        cache.set("product:1", "Phone", None);

        let users = cache.fuzzy_find("user:");
        assert_eq!(users.len(), 2);

        let products = cache.fuzzy_find("product");
        assert_eq!(products.len(), 1);
    }

    #[test]
    fn test_fuzzy_find_regex() {
        let cache = BackCache::new();

        cache.set("user_1", "Alice", None);
        cache.set("user_2", "Bob", None);
        cache.set("product_1", "Phone", None);

        let users = cache.fuzzy_find_regex(r"user_\d+").unwrap();
        assert_eq!(users.len(), 2);

        let numbers = cache.fuzzy_find_regex(r"\d").unwrap();
        assert_eq!(numbers.len(), 3);
    }

    #[test]
    fn test_cleanup_task() {
        let cache = BackCache::new();

        // Fill with expired items
        for i in 0..100 {
            cache.set(format!("temp{}", i), i, Some(Duration::from_nanos(1)));
        }

        // Let cleanup task run
        thread::sleep(Duration::from_millis(150));

        // All expired items should be removed
        assert!(cache.len() < 100);
    }

    #[test]
    fn test_clear() {
        let cache = BackCache::new();

        cache.set("key1", "value1", None);
        cache.set("key2", "value2", None);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_any() {
       
        let cache = BackCache::new();
        cache.set(
            "key1",
            CacheObject(String::from("Hello, world!")),
            Some(Duration::from_secs(100)),
        );
        let value = cache.get("key1");
        assert_eq!(value.map(|f| f.as_object::<String>()).unwrap().unwrap(), "Hello, world!".to_string());


        #[derive(Clone, Debug)]
        struct TestA(pub String);

        let gogo = TestA("I am Gogo!!".to_string());
        cache.set("test_gogo", CacheObject(gogo), None);
        if let Some(value) = cache.get("test_gogo") {
            if let Some(var) = value.as_object::<TestA>() {
                assert_eq!(var.0, "I am Gogo!!".to_string());
            }
        }
    }
}
