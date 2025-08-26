use std::any::Any;
use std::fmt::{self};
use std::sync::RwLock;

use hashbrown::HashMap;
use std::borrow::Cow;
use std::sync::Arc;

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

/// 缓存值类型枚举
///
/// Cache value type enum
#[derive(Clone)]
pub enum AnyValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Float(f32),
    Array(Vec<AnyValue>),
    Object(Box<dyn AnyClone>),
    Null,
}

/// 可克隆的任意类型trait
/// Cloneable any type trait
pub trait AnyClone: Any + Send + Sync {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn clone_box(&self) -> Box<dyn AnyClone>;
}

/// 为所有实现了Clone+Send+Sync的类型实现AnyClone
/// Implement AnyClone for all types that implement Clone+Send+Sync
impl<T> AnyClone for T
where
    T: Any + Clone + Send + Sync,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
}

impl fmt::Debug for dyn AnyClone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Implement Clone for Box<dyn AnyClone>
impl Clone for Box<dyn AnyClone> {
    fn clone(&self) -> Box<dyn AnyClone> {
        (**self).clone_box()
    }
}

impl fmt::Debug for AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyValue::String(s) => write!(f, "String({:?})", s),
            AnyValue::Number(n) => write!(f, "Number({})", n),
            AnyValue::Boolean(b) => write!(f, "Boolean({})", b),
            AnyValue::Float(num) => write!(f, "Float({})", num),
            AnyValue::Array(arr) => write!(f, "Array({:?})", arr),
            AnyValue::Object(obj) => write!(f, "Object({:?})", obj),
            AnyValue::Null => write!(f, "Null"),
        }
    }
}

/// 缓存对象包装器
/// Cache object wrapper
pub struct CacheObject<T>(pub T)
where
    T: Any + Clone + Send + Sync;

impl AnyValue {
    /// 检查是否为数字类型
    /// Check if the value is a number
    pub fn is_number(&self) -> bool {
        matches!(self, AnyValue::Number(_))
    }

    /// 检查是否为浮点数类型
    /// Check if the value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, AnyValue::Float(_))
    }

    /// 检查是否为字符串类型
    /// Check if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, AnyValue::String(_))
    }

    /// 检查是否为布尔类型
    /// Check if the value is a boolean
    pub fn is_boolean(&self) -> bool {
        matches!(self, AnyValue::Boolean(_))
    }

    /// 检查是否为null
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, AnyValue::Null)
    }

    /// 检查是否为数组类型
    /// Check if the value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, AnyValue::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, AnyValue::Object(_))
    }

    /// 获取字符串值
    /// Get string value
    pub fn as_string(&self) -> Option<String> {
        if let AnyValue::String(s) = self {
            Some(s.to_owned())
        } else {
            None
        }
    }

    /// 获取数字值
    /// Get number value
    pub fn as_number(&self) -> Option<i64> {
        if let AnyValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// 获取浮点数
    /// Get float value
    pub fn as_float(&self) -> Option<f32> {
        if let AnyValue::Float(f) = self {
            Some(*f)
        } else {
            None
        }
    }

    /// 获取布尔值
    /// Get boolean value
    pub fn as_boolean(&self) -> Option<bool> {
        if let AnyValue::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// 获取数组引用
    /// Get array reference
    pub fn as_array(&self) -> Option<&Vec<AnyValue>> {
        if let AnyValue::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }

    /// 转换为字符串表示
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            AnyValue::String(s) => s.clone(),
            AnyValue::Number(n) => n.to_string(),
            AnyValue::Boolean(b) => b.to_string(),
            AnyValue::Null => "null".to_string(),
            AnyValue::Array(a) => {
                let mut s = String::new();
                s.push('[');

                for v in a.iter() {
                    s.push_str(&v.to_string());
                    s.push(',');
                }
                s.push(']');
                s
            }
            _ => "".to_string(),
        }
    }

    pub fn as_object<T: Any>(&self) -> Option<T> {
        if let AnyValue::Object(obj) = self {
            let any_obj = obj.clone();
            any_obj.into_any().downcast().map(|obj| *obj).ok()
        } else {
            None
        }
    }
}

impl Into<AnyValue> for String {
    fn into(self) -> AnyValue {
        AnyValue::String(self)
    }
}

impl Into<AnyValue> for &str {
    fn into(self) -> AnyValue {
        AnyValue::String(self.to_string())
    }
}

impl Into<AnyValue> for i32 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self as i64)
    }
}

impl Into<AnyValue> for i64 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self)
    }
}

impl Into<AnyValue> for u32 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self as i64)
    }
}

impl Into<AnyValue> for f32 {
    fn into(self) -> AnyValue {
        AnyValue::Float(self)
    }
}

impl Into<AnyValue> for bool {
    fn into(self) -> AnyValue {
        AnyValue::Boolean(self)
    }
}

impl Into<AnyValue> for Vec<AnyValue> {
    fn into(self) -> AnyValue {
        AnyValue::Array(self)
    }
}

impl<T: 'static + Any + Clone + Send + Sync> Into<AnyValue> for CacheObject<T> {
    fn into(self) -> AnyValue {
        AnyValue::Object(Box::new(self.0))
    }
}
