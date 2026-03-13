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
    /// Check if the value is a number
    ///
    /// 检查是否为数字类型
    pub fn is_number(&self) -> bool {
        matches!(self, AnyValue::Number(_))
    }

    /// Check if the value is a float
    ///
    /// 检查是否为浮点数类型
    pub fn is_float(&self) -> bool {
        matches!(self, AnyValue::Float(_))
    }

    /// Check if the value is a string
    ///
    /// 检查是否为字符串类型
    pub fn is_string(&self) -> bool {
        matches!(self, AnyValue::String(_))
    }

    /// Check if the value is a boolean
    ///
    /// 检查是否为布尔类型
    pub fn is_boolean(&self) -> bool {
        matches!(self, AnyValue::Boolean(_))
    }

    /// Check if the value is null
    ///
    /// 检查是否为null
    pub fn is_null(&self) -> bool {
        matches!(self, AnyValue::Null)
    }

    /// Check if the value is a map
    ///
    /// 检查是否为map类型
    pub fn is_map(&self) -> bool {
        matches!(self, AnyValue::Map(_))
    }

    /// Check if the value is an array
    ///
    /// 检查是否为数组类型
    pub fn is_list(&self) -> bool {
        matches!(self, AnyValue::List(_))
    }

    /// Check if the value is an object
    ///
    /// 检查是否为对象类型
    pub fn is_object(&self) -> bool {
        matches!(self, AnyValue::Object(_))
    }

<<<<<<< HEAD
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
=======
>>>>>>> v0.2.0
    /// Get string value
    ///
    /// 获取字符串值
    pub fn as_string(&self) -> Option<String> {
        if let AnyValue::String(value) = self {
            Some(value.clone())
        } else {
            None
        }
    }

    /// Get string value
    ///
    /// 获取字符串值
    pub fn as_str(&self) -> Option<&str> {
        if let AnyValue::String(value) = self {
            Some(value.as_str())
        } else {
            None
        }
    }

    /// Get number value
    ///
    /// 获取数字值
    pub fn as_number(&self) -> Option<i64> {
        if let AnyValue::Number(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Get float value
    ///
    /// 获取浮点数
    pub fn as_float(&self) -> Option<f64> {
        if let AnyValue::Float(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Get boolean value
    ///
    /// 获取布尔值
    pub fn as_boolean(&self) -> Option<bool> {
        if let AnyValue::Boolean(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Get map value
    ///
    /// 获取映射
    pub fn as_map(&self) -> Option<&HashMap<String, AnyValue>> {
        if let AnyValue::Map(value) = self {
            Some(value)
        } else {
            None
        }
    }

    /// Get list reference
    ///
    /// 获取数组引用
    pub fn as_list(&self) -> Option<&Vec<AnyValue>> {
        if let AnyValue::List(a) = self {
            Some(a)
        } else {
            None
        }
    }

    /// Get object reference
    ///
    /// 获取对象
    pub fn as_object<T: Any>(&self) -> Option<T> {
        if let AnyValue::Object(obj) = self {
            let any_obj = obj.clone();
            any_obj.into_any().downcast().map(|obj| *obj).ok()
        } else {
            None
        }
    }

    /// Get object reference
    ///
    /// 获取对象引用
    pub fn as_ref_object<T: Any>(&self) -> Option<&T> {
        if let AnyValue::Object(obj) = self {
            let any_obj: &dyn Any = obj;
            any_obj.downcast_ref()
        } else {
            None
        }
    }

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
<<<<<<< HEAD

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
=======
>>>>>>> v0.2.0
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

impl Into<AnyValue> for i8 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self as i64)
    }
}

impl Into<AnyValue> for i16 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self as i64)
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

impl Into<AnyValue> for u8 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self as i64)
    }
}

impl Into<AnyValue> for u16 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self as i64)
    }
}

impl Into<AnyValue> for u32 {
    fn into(self) -> AnyValue {
        AnyValue::Number(self as i64)
    }
}

impl Into<AnyValue> for u64 {
    fn into(self) -> AnyValue {
        if self <= i64::MAX as u64 {
            AnyValue::Number(self as i64)
        } else {
            panic!("u64 value {} cannot fit into i64", self);
        }
    }
}

impl Into<AnyValue> for f32 {
    fn into(self) -> AnyValue {
        AnyValue::Float(self as f64)
    }
}

impl Into<AnyValue> for f64 {
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
