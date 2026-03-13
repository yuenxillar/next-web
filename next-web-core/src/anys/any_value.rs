use std::{any::Any, collections::HashMap, fmt};

use crate::traits::any_clone::AnyClone;

#[derive(Clone, Default)]
pub enum AnyValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Float(f64),
    Map(HashMap<String, AnyValue>),
    List(Vec<AnyValue>),
    Object(Box<dyn AnyClone>),
    #[default]
    Null,
}

impl AnyValue {
    /// 检查是否为数字类型
    ///
    /// Check if the value is a number
    pub fn is_number(&self) -> bool {
        matches!(self, AnyValue::Number(_))
    }

    /// 检查是否为浮点数类型
    ///
    /// Check if the value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, AnyValue::Float(_))
    }

    /// 检查是否为字符串类型
    ///
    /// Check if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, AnyValue::String(_))
    }

    /// 检查是否为布尔类型
    ///
    /// Check if the value is a boolean
    pub fn is_boolean(&self) -> bool {
        matches!(self, AnyValue::Boolean(_))
    }

    /// 检查是否为null
    ///
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, AnyValue::Null)
    }

    /// 检查是否为map类型
    ///
    /// Check if the value is a map
    pub fn is_map(&self) -> bool {
        matches!(self, AnyValue::Map(_))
    }

    /// 检查是否为数组类型
    ///
    /// Check if the value is an array
    pub fn is_list(&self) -> bool {
        matches!(self, AnyValue::List(_))
    }

    /// 检查是否为对象类型
    ///
    /// Check if the value is an object
    pub fn is_object(&self) -> bool {
        matches!(self, AnyValue::Object(_))
    }

    /// 检查是否为对象类型
    pub fn is_object_type<T: Any>(&self) -> bool {
        if let AnyValue::Object(obj) = self {
            let any_obj: &dyn Any = obj;
            return any_obj.downcast_ref::<T>().is_some();
        }

        false
    }

    /// 获取字符串值
    ///
    /// Get string value
    pub fn as_string(&self) -> Option<String> {
        if let AnyValue::String(s) = self {
            Some(s.clone())
        } else {
            None
        }
    }

    /// 获取字符串值
    ///
    /// Get string value
    pub fn as_str(&self) -> Option<&str> {
        if let AnyValue::String(s) = self {
            Some(s.as_str())
        } else {
            None
        }
    }

    /// 获取数字值
    ///
    /// Get number value
    pub fn as_number(&self) -> Option<i64> {
        if let AnyValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// 获取浮点数
    ///
    /// Get float value
    pub fn as_float(&self) -> Option<f64> {
        if let AnyValue::Float(f) = self {
            Some(*f)
        } else {
            None
        }
    }

    /// 获取布尔值
    ///
    /// Get boolean value
    pub fn as_boolean(&self) -> Option<bool> {
        if let AnyValue::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// 获取映射
    ///
    /// Get map value
    pub fn as_map(&self) -> Option<&HashMap<String, AnyValue>> {
        if let AnyValue::Map(m) = self {
            Some(m)
        } else {
            None
        }
    }

    /// 获取数组引用
    ///
    /// Get list reference
    pub fn as_list(&self) -> Option<&Vec<AnyValue>> {
        if let AnyValue::List(a) = self {
            Some(a)
        } else {
            None
        }
    }

    /// 转换为字符串表示
    ///
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            AnyValue::String(s) => s.clone(),
            AnyValue::Number(n) => n.to_string(),
            AnyValue::Boolean(b) => b.to_string(),
            AnyValue::Null => "null".to_string(),
            AnyValue::List(a) => {
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

    pub fn as_ref_object<T: Any>(&self) -> Option<&T> {
        if let AnyValue::Object(obj) = self {
            let any_obj: &dyn Any = obj;
            any_obj.downcast_ref()
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

impl Into<AnyValue> for u16 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self as i64)
    }
}

impl Into<AnyValue> for f32 {
    fn into(self) -> AnyValue {
        AnyValue::Float(self as f64)
    }
}

impl Into<AnyValue> for bool {
    fn into(self) -> AnyValue {
        AnyValue::Boolean(self)
    }
}

impl Into<AnyValue> for Vec<AnyValue> {
    fn into(self) -> AnyValue {
        AnyValue::List(self)
    }
}

impl fmt::Debug for AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyValue::String(s) => write!(f, "String({:?})", s),
            AnyValue::Number(n) => write!(f, "Number({})", n),
            AnyValue::Boolean(b) => write!(f, "Boolean({})", b),
            AnyValue::Float(num) => write!(f, "Float({})", num),
            AnyValue::Map(map) => write!(f, "Map({:?})", map),
            AnyValue::List(arr) => write!(f, "List({:?})", arr),
            AnyValue::Object(obj) => write!(f, "Object({:?})", obj),
            AnyValue::Null => write!(f, "Null"),
        }
    }
}
