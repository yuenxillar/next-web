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
    /// Check if the value is a number.
    pub fn is_number(&self) -> bool {
        matches!(self, AnyValue::Number(_))
    }

    /// Check if the value is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, AnyValue::Float(_))
    }

    /// Check if the value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, AnyValue::String(_))
    }

    /// Check if the value is a boolean.
    pub fn is_boolean(&self) -> bool {
        matches!(self, AnyValue::Boolean(_))
    }

    /// Check if the value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, AnyValue::Null)
    }

    /// Check if the value is a map.
    pub fn is_map(&self) -> bool {
        matches!(self, AnyValue::Map(_))
    }

    /// Check if the value is a list.
    pub fn is_list(&self) -> bool {
        matches!(self, AnyValue::List(_))
    }

    /// Check if the value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, AnyValue::Object(_))
    }

    /// Get a cloned string value.
    pub fn as_string(&self) -> Option<String> {
        if let AnyValue::String(value) = self {
            Some(value.clone())
        } else {
            None
        }
    }

    /// Get a borrowed string value.
    pub fn as_str(&self) -> Option<&str> {
        if let AnyValue::String(value) = self {
            Some(value.as_str())
        } else {
            None
        }
    }

    /// Get a number value.
    pub fn as_number(&self) -> Option<i64> {
        if let AnyValue::Number(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Get a float value.
    pub fn as_float(&self) -> Option<f64> {
        if let AnyValue::Float(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Get a boolean value.
    pub fn as_boolean(&self) -> Option<bool> {
        if let AnyValue::Boolean(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Get a map reference.
    pub fn as_map(&self) -> Option<&HashMap<String, AnyValue>> {
        if let AnyValue::Map(value) = self {
            Some(value)
        } else {
            None
        }
    }

    /// Get a list reference.
    pub fn as_list(&self) -> Option<&Vec<AnyValue>> {
        if let AnyValue::List(value) = self {
            Some(value)
        } else {
            None
        }
    }

    /// Get a cloned object value.
    ///
    /// This clones the stored object before downcasting it. Use
    /// [`Self::as_ref_object`] to borrow the object without cloning, or
    /// [`Self::as_own_object`] when consuming this `AnyValue`.
    pub fn as_object<T: Any>(&self) -> Option<T> {
        if let AnyValue::Object(obj) = self {
            let any_obj = obj.clone();
            any_obj.into_any().downcast().map(|obj| *obj).ok()
        } else {
            None
        }
    }

    /// Get an object reference.
    pub fn as_ref_object<T: Any>(&self) -> Option<&T> {
        if let AnyValue::Object(obj) = self {
            let any_obj: &dyn Any = obj.as_ref();
            any_obj.downcast_ref()
        } else {
            None
        }
    }

    /// Get a mutable reference to the object value.
    pub fn as_mut_object<T: Any>(&mut self) -> Option<&mut T> {
        if let AnyValue::Object(obj) = self {
            let any_obj: &mut dyn Any = obj.as_mut();
            any_obj.downcast_mut()
        } else {
            None
        }
    }

    /// Get an owned object by consuming this value.
    pub fn as_own_object<T: Any>(self) -> Option<T> {
        if let AnyValue::Object(any_obj) = self {
            any_obj.into_any().downcast().map(|obj| *obj).ok()
        } else {
            None
        }
    }

    /// Convert to a string representation.
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        ToString::to_string(self)
    }
}

impl From<String> for AnyValue {
    fn from(value: String) -> Self {
        AnyValue::String(value)
    }
}

impl From<&str> for AnyValue {
    fn from(value: &str) -> Self {
        AnyValue::String(value.to_string())
    }
}

impl From<i8> for AnyValue {
    fn from(value: i8) -> Self {
        AnyValue::Number(value as i64)
    }
}

impl From<i16> for AnyValue {
    fn from(value: i16) -> Self {
        AnyValue::Number(value as i64)
    }
}

impl From<i32> for AnyValue {
    fn from(value: i32) -> Self {
        AnyValue::Number(value as i64)
    }
}

impl From<i64> for AnyValue {
    fn from(value: i64) -> Self {
        AnyValue::Number(value)
    }
}

impl From<u8> for AnyValue {
    fn from(value: u8) -> Self {
        AnyValue::Number(value as i64)
    }
}

impl From<u16> for AnyValue {
    fn from(value: u16) -> Self {
        AnyValue::Number(value as i64)
    }
}

impl From<u32> for AnyValue {
    fn from(value: u32) -> Self {
        AnyValue::Number(value as i64)
    }
}

impl From<u64> for AnyValue {
    /// Converts a `u64` to `AnyValue::Number`.
    ///
    /// # Panics
    ///
    /// Panics when the value is larger than `i64::MAX`.
    fn from(value: u64) -> Self {
        if value <= i64::MAX as u64 {
            AnyValue::Number(value as i64)
        } else {
            panic!("u64 value {value} cannot fit into i64");
        }
    }
}

impl From<f32> for AnyValue {
    fn from(value: f32) -> Self {
        AnyValue::Float(value as f64)
    }
}

impl From<f64> for AnyValue {
    fn from(value: f64) -> Self {
        AnyValue::Float(value)
    }
}

impl From<bool> for AnyValue {
    fn from(value: bool) -> Self {
        AnyValue::Boolean(value)
    }
}

impl From<Vec<AnyValue>> for AnyValue {
    fn from(value: Vec<AnyValue>) -> Self {
        AnyValue::List(value)
    }
}

impl From<HashMap<String, AnyValue>> for AnyValue {
    fn from(value: HashMap<String, AnyValue>) -> Self {
        AnyValue::Map(value)
    }
}

impl fmt::Display for AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyValue::String(value) => write!(f, "{value}"),
            AnyValue::Number(value) => write!(f, "{value}"),
            AnyValue::Boolean(value) => write!(f, "{value}"),
            AnyValue::Float(value) => write!(f, "{value}"),
            AnyValue::Map(value) => write!(f, "{value:?}"),
            AnyValue::List(value) => {
                write!(f, "[")?;
                for (index, item) in value.iter().enumerate() {
                    if index > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            AnyValue::Object(_) => write!(f, "<object>"),
            AnyValue::Null => write!(f, "null"),
        }
    }
}

impl fmt::Debug for AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyValue::String(value) => write!(f, "String({value:?})"),
            AnyValue::Number(value) => write!(f, "Number({value})"),
            AnyValue::Boolean(value) => write!(f, "Boolean({value})"),
            AnyValue::Float(value) => write!(f, "Float({value})"),
            AnyValue::Map(value) => write!(f, "Map({value:?})"),
            AnyValue::List(value) => write!(f, "List({value:?})"),
            AnyValue::Object(_) => write!(f, "Object(<object>)"),
            AnyValue::Null => write!(f, "Null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::AnyValue;

    #[test]
    fn display_and_to_string_cover_supported_variants() {
        assert_eq!(AnyValue::from("hello").to_string(), "hello");
        assert_eq!(AnyValue::from(42_i64).to_string(), "42");
        assert_eq!(AnyValue::from(1.5_f64).to_string(), "1.5");
        assert_eq!(AnyValue::from(true).to_string(), "true");
        assert_eq!(AnyValue::Null.to_string(), "null");

        let list = AnyValue::from(vec![AnyValue::from(1_i64), AnyValue::from("two")]);
        assert_eq!(list.to_string(), "[1,two]");

        let mut map = HashMap::new();
        map.insert("key".to_string(), AnyValue::from("value"));
        let map_value = AnyValue::from(map);
        assert!(map_value.to_string().contains("key"));
        assert!(map_value.to_string().contains("value"));
    }

    #[test]
    fn object_accessors_support_clone_borrow_and_owned_access() {
        let value = AnyValue::Object(Box::new(String::from("payload")));

        assert_eq!(
            value.as_ref_object::<String>(),
            Some(&String::from("payload"))
        );
        assert_eq!(value.as_object::<String>(), Some(String::from("payload")));
        assert_eq!(
            value.as_own_object::<String>(),
            Some(String::from("payload"))
        );
    }
}
