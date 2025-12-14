use std::any::Any;

/// 缓存值类型枚举
/// Cache value type enum
#[derive(Clone)]
pub enum CacheValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Float(f32),
    Array(Vec<CacheValue>),
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
    T:  Any + Clone + Send + Sync,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
}

// Implement Clone for Box<dyn AnyClone>
impl Clone for Box<dyn AnyClone> {
    fn clone(&self) -> Box<dyn AnyClone> {
        (**self).clone_box()
    }
}

/// 缓存对象包装器
/// Cache object wrapper
pub struct CacheObject<T>(pub T)
where
    T: Any + Clone + Send + Sync;

impl CacheValue {
    /// 检查是否为数字类型
    /// Check if the value is a number
    pub fn is_number(&self) -> bool {
        matches!(self, CacheValue::Number(_))
    }

    /// 检查是否为浮点数类型
    /// Check if the value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, CacheValue::Float(_))
    }

    /// 检查是否为字符串类型
    /// Check if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, CacheValue::String(_))
    }

    /// 检查是否为布尔类型
    /// Check if the value is a boolean
    pub fn is_boolean(&self) -> bool {
        matches!(self, CacheValue::Boolean(_))
    }

    /// 检查是否为null
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, CacheValue::Null)
    }

    /// 检查是否为数组类型
    /// Check if the value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, CacheValue::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, CacheValue::Object(_))
    }

    /// 获取字符串值
    /// Get string value
    pub fn as_string(&self) -> Option<String> {
        if let CacheValue::String(s) = self {
            Some(s.to_owned())
        } else {
            None
        }
    }

    /// 获取数字值
    /// Get number value
    pub fn as_number(&self) -> Option<i64> {
        if let CacheValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// 获取浮点数
    /// Get float value
    pub fn as_float(&self) -> Option<f32> {
        if let CacheValue::Float(f) = self {
            Some(*f)
        } else {
            None
        }
    }

    /// 获取布尔值
    /// Get boolean value
    pub fn as_boolean(&self) -> Option<bool> {
        if let CacheValue::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// 获取数组引用
    /// Get array reference
    pub fn as_array(&self) -> Option<&Vec<CacheValue>> {
        if let CacheValue::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }

    /// 转换为字符串表示
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            CacheValue::String(s) => s.clone(),
            CacheValue::Number(n) => n.to_string(),
            CacheValue::Boolean(b) => b.to_string(),
            CacheValue::Null => "null".to_string(),
            CacheValue::Array(a) => {
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
        if let CacheValue::Object(obj) = self {
            let any_obj = obj.clone();
            any_obj.into_any().downcast().map(|obj| *obj).ok()
        } else {
            None
        }
    }
}

impl Into<CacheValue> for String {
    fn into(self) -> CacheValue {
        CacheValue::String(self)
    }
}

impl Into<CacheValue> for &str {
    fn into(self) -> CacheValue {
        CacheValue::String(self.to_string())
    }
}

impl Into<CacheValue> for i32 {
    fn into(self) -> CacheValue {
        CacheValue::Number(self as i64)
    }
}

impl Into<CacheValue> for i64 {
    fn into(self) -> CacheValue {
        CacheValue::Number(self)
    }
}

impl Into<CacheValue> for u32 {
    fn into(self) -> CacheValue {
        CacheValue::Number(self as i64)
    }
}

impl Into<CacheValue> for f32 {
    fn into(self) -> CacheValue {
        CacheValue::Float(self)
    }
}

impl Into<CacheValue> for bool {
    fn into(self) -> CacheValue {
        CacheValue::Boolean(self)
    }
}

impl Into<CacheValue> for Vec<CacheValue> {
    fn into(self) -> CacheValue {
        CacheValue::Array(self)
    }
}

impl<T: 'static + Any + Clone + Send + Sync> Into<CacheValue> for CacheObject<T> {
    fn into(self) -> CacheValue {
        CacheValue::Object(Box::new(self.0))
    }
}