use std::{any::Any, collections::HashMap, fmt};

use crate::traits::any_clone::AnyClone;

/// A numeric value that preserves its signedness and float-ness.
///
/// Integers are stored in the widest type of their signedness class:
/// all signed integers (`i8`..=`i128`) become [`Number::Int`], all
/// unsigned integers (`u8`..=`u128`) become [`Number::UInt`], and both
/// `f32` and `f64` become [`Number::Float`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Number {
    /// A signed integer, covering `i8` through `i128`.
    Int(i128),
    /// An unsigned integer, covering `u8` through `u128`.
    UInt(u128),
    /// A floating point number, covering `f32` and `f64`.
    Float(f64),
}

/// A dynamically typed value.
///
/// `AnyValue` is a lightweight sum type used to represent configuration
/// values, JSON-like data, or any heterogeneous payload. It supports
/// strings, numbers, booleans, maps, lists, and type-erased objects.
///
/// # Object variant
///
/// [`AnyValue::BoxedValue`] stores a `Box<dyn AnyClone>`, which allows cloning
/// the value but not direct equality comparison. Because of this,
/// `AnyValue` intentionally does **not** implement [`PartialEq`].
#[derive(Clone, Default)]
pub enum AnyValue {
    /// A UTF-8 string.
    String(String),
    /// A numeric value; see [`Number`] for the exact representation.
    Number(Number),
    /// A boolean.
    Boolean(bool),
    /// A map from string keys to values.
    Map(HashMap<String, AnyValue>),
    /// An ordered list of values.
    List(Vec<AnyValue>),
    /// A type-erased object that can be cloned.
    BoxedValue(Box<dyn AnyClone>),

    /// The absence of a value.
    #[default]
    Null,
}

impl AnyValue {
    /// Returns `true` if this is a signed or unsigned integer.
    ///
    /// Floating point values are reported by [`Self::is_float`], not here.
    #[must_use]
    pub fn is_number(&self) -> bool {
        matches!(self, AnyValue::Number(Number::Int(_) | Number::UInt(_)))
    }

    /// Returns `true` if this is a floating point number.
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(self, AnyValue::Number(Number::Float(_)))
    }

    /// Returns `true` if this is a string.
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, AnyValue::String(_))
    }

    /// Returns `true` if this is a boolean.
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, AnyValue::Boolean(_))
    }

    /// Returns `true` if this is [`AnyValue::Null`].
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, AnyValue::Null)
    }

    /// Returns `true` if this is a map.
    #[must_use]
    pub fn is_map(&self) -> bool {
        matches!(self, AnyValue::Map(_))
    }

    /// Returns `true` if this is a list.
    #[must_use]
    pub fn is_list(&self) -> bool {
        matches!(self, AnyValue::List(_))
    }

    /// Returns `true` if this is a type-erased object.
    #[must_use]
    pub fn is_value(&self) -> bool {
        matches!(self, AnyValue::BoxedValue(_))
    }

    /// Returns a cloned `String` if this is a string value.
    ///
    /// This allocates. Prefer [`Self::as_str`] when a borrow is enough.
    #[must_use]
    pub fn as_string(&self) -> Option<String> {
        match self {
            AnyValue::String(value) => Some(value.clone()),
            _ => None,
        }
    }

    /// Returns a borrowed `&str` if this is a string value.
    ///
    /// This is allocation-free and suitable for hot paths.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AnyValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    }

    /// Returns the value as `i64` if it fits.
    ///
    /// Signed integers within `i64` range return `Some`; unsigned integers
    /// within `i64` range also return `Some`. Everything else, including
    /// out-of-range values and floats, returns `None`.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            AnyValue::Number(Number::Int(value)) => i64::try_from(*value).ok(),
            AnyValue::Number(Number::UInt(value)) => i64::try_from(*value).ok(),
            _ => None,
        }
    }

    /// Returns the value as `u64` if it fits.
    ///
    /// Unsigned integers within `u64` range return `Some`; non-negative
    /// signed integers within `u64` range also return `Some`. Everything
    /// else returns `None`.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            AnyValue::Number(Number::Int(value)) => u64::try_from(*value).ok(),
            AnyValue::Number(Number::UInt(value)) => u64::try_from(*value).ok(),
            _ => None,
        }
    }

    /// Returns the value as `i128` if it is an integer.
    ///
    /// Both signed and unsigned integers are converted when they fit.
    #[must_use]
    pub fn as_i128(&self) -> Option<i128> {
        match self {
            AnyValue::Number(Number::Int(value)) => Some(*value),
            AnyValue::Number(Number::UInt(value)) => i128::try_from(*value).ok(),
            _ => None,
        }
    }

    /// Returns the value as `u128` if it is a non-negative integer.
    #[must_use]
    pub fn as_u128(&self) -> Option<u128> {
        match self {
            AnyValue::Number(Number::Int(value)) => u128::try_from(*value).ok(),
            AnyValue::Number(Number::UInt(value)) => Some(*value),
            _ => None,
        }
    }

    /// Returns the value as `f64` if it is a float.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            AnyValue::Number(Number::Float(value)) => Some(*value),
            _ => None,
        }
    }

    /// Returns the value as `bool` if it is a boolean.
    #[must_use]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            AnyValue::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns a reference to the underlying map, if any.
    #[must_use]
    pub fn as_map(&self) -> Option<&HashMap<String, AnyValue>> {
        match self {
            AnyValue::Map(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying map, if any.
    #[must_use]
    pub fn as_mut_map(&mut self) -> Option<&mut HashMap<String, AnyValue>> {
        match self {
            AnyValue::Map(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a reference to the underlying list, if any.
    #[must_use]
    pub fn as_list(&self) -> Option<&Vec<AnyValue>> {
        match self {
            AnyValue::List(value) => Some(value),
            _ => None,
        }
    }

    /// Returns a mutable reference to the underlying list, if any.
    #[must_use]
    pub fn as_mut_list(&mut self) -> Option<&mut Vec<AnyValue>> {
        match self {
            AnyValue::List(value) => Some(value),
            _ => None,
        }
    }

    /// Clones the stored object and downcasts it to `T`.
    ///
    /// This allocates and clones the object. Prefer [`Self::as_ref_value`]
    /// to borrow without cloning, or [`Self::as_own_value`] when consuming
    /// this `AnyValue`.
    ///
    /// Returns `None` if this is not an object or the downcast fails.
    #[must_use]
    pub fn to_value<T: Any>(&self) -> Option<T> {
        match self {
            AnyValue::BoxedValue(obj) => {
                let any_obj = obj.clone();
                any_obj.into_any().downcast().map(|obj| *obj).ok()
            }
            _ => None,
        }
    }

    /// Returns a borrowed reference to the stored object as `T`.
    ///
    /// This is allocation-free. Returns `None` if this is not an object or
    /// the downcast fails.
    #[must_use]
    pub fn as_ref_value<T: Any>(&self) -> Option<&T> {
        match self {
            AnyValue::BoxedValue(obj) => {
                let any_obj: &dyn Any = obj.as_ref();
                any_obj.downcast_ref()
            }
            _ => None,
        }
    }

    /// Returns a mutable reference to the stored object as `T`.
    ///
    /// Returns `None` if this is not an object or the downcast fails.
    #[must_use]
    pub fn as_mut_value<T: Any>(&mut self) -> Option<&mut T> {
        match self {
            AnyValue::BoxedValue(obj) => {
                let any_obj: &mut dyn Any = obj.as_mut();
                any_obj.downcast_mut()
            }
            _ => None,
        }
    }

    /// Consumes this value and returns the stored object as `T`.
    ///
    /// This is allocation-free. Returns `None` if this is not an object or
    /// the downcast fails.
    #[must_use]
    pub fn as_own_value<T: Any>(self) -> Option<T> {
        match self {
            AnyValue::BoxedValue(any_obj) => any_obj.into_any().downcast().map(|obj| *obj).ok(),
            _ => None,
        }
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
        AnyValue::Number(Number::Int(i128::from(value)))
    }
}

impl From<i16> for AnyValue {
    fn from(value: i16) -> Self {
        AnyValue::Number(Number::Int(i128::from(value)))
    }
}

impl From<i32> for AnyValue {
    fn from(value: i32) -> Self {
        AnyValue::Number(Number::Int(i128::from(value)))
    }
}

impl From<i64> for AnyValue {
    fn from(value: i64) -> Self {
        AnyValue::Number(Number::Int(i128::from(value)))
    }
}

impl From<i128> for AnyValue {
    fn from(value: i128) -> Self {
        AnyValue::Number(Number::Int(value))
    }
}

impl From<isize> for AnyValue {
    fn from(value: isize) -> Self {
        AnyValue::Number(Number::Int(value as i128))
    }
}

impl From<u8> for AnyValue {
    fn from(value: u8) -> Self {
        AnyValue::Number(Number::UInt(u128::from(value)))
    }
}

impl From<u16> for AnyValue {
    fn from(value: u16) -> Self {
        AnyValue::Number(Number::UInt(u128::from(value)))
    }
}

impl From<u32> for AnyValue {
    fn from(value: u32) -> Self {
        AnyValue::Number(Number::UInt(u128::from(value)))
    }
}

impl From<u64> for AnyValue {
    fn from(value: u64) -> Self {
        AnyValue::Number(Number::UInt(u128::from(value)))
    }
}

impl From<u128> for AnyValue {
    fn from(value: u128) -> Self {
        AnyValue::Number(Number::UInt(value))
    }
}

impl From<usize> for AnyValue {
    fn from(value: usize) -> Self {
        AnyValue::Number(Number::UInt(value as u128))
    }
}

impl From<f32> for AnyValue {
    fn from(value: f32) -> Self {
        AnyValue::Number(Number::Float(f64::from(value)))
    }
}

impl From<f64> for AnyValue {
    fn from(value: f64) -> Self {
        AnyValue::Number(Number::Float(value))
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
            AnyValue::Number(Number::Int(value)) => write!(f, "{value}"),
            AnyValue::Number(Number::UInt(value)) => write!(f, "{value}"),
            AnyValue::Number(Number::Float(value)) => write!(f, "{value}"),
            AnyValue::Boolean(value) => write!(f, "{value}"),
            // `HashMap` iteration order is unspecified, so the debug
            // representation is used to avoid promising a stable order.
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
            AnyValue::BoxedValue(_) => write!(f, "<value>"),
            AnyValue::Null => write!(f, "null"),
        }
    }
}

impl fmt::Debug for AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnyValue::String(value) => write!(f, "String({value:?})"),
            AnyValue::Number(Number::Int(value)) => write!(f, "Int({value})"),
            AnyValue::Number(Number::UInt(value)) => write!(f, "UInt({value})"),
            AnyValue::Number(Number::Float(value)) => write!(f, "Float({value})"),
            AnyValue::Boolean(value) => write!(f, "Boolean({value})"),
            AnyValue::Map(value) => write!(f, "Map({value:?})"),
            AnyValue::List(value) => write!(f, "List({value:?})"),
            AnyValue::BoxedValue(_) => write!(f, "BoxedValue(<value>)"),
            AnyValue::Null => write!(f, "Null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{AnyValue, Number};

    #[test]
    fn display_covers_supported_variants() {
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
    fn large_integers_are_supported() {
        let big = AnyValue::from(i128::MAX);
        assert_eq!(big.as_i128(), Some(i128::MAX));
        assert!(big.is_number());

        let huge = AnyValue::from(u128::MAX);
        assert_eq!(huge.as_u128(), Some(u128::MAX));
        assert!(huge.is_number());

        // `u64::MAX` no longer panics and fits into `u128`.
        let u64_max = AnyValue::from(u64::MAX);
        assert_eq!(u64_max.as_u128(), Some(u128::from(u64::MAX)));
    }

    #[test]
    fn numeric_accessors_respect_ranges() {
        let value = AnyValue::from(300_u32);
        assert_eq!(value.as_i64(), Some(300));
        assert_eq!(value.as_u64(), Some(300));
        assert_eq!(value.as_i128(), Some(300));
        assert_eq!(value.as_u128(), Some(300));

        let negative = AnyValue::from(-1_i32);
        assert_eq!(negative.as_i64(), Some(-1));
        assert_eq!(negative.as_u64(), None);
        assert_eq!(negative.as_u128(), None);

        let float = AnyValue::from(1.5_f64);
        assert_eq!(float.as_f64(), Some(1.5));
        assert!(!float.is_number());
        assert!(float.is_float());
    }

    #[test]
    fn object_accessors_support_clone_borrow_and_owned_access() {
        let value = AnyValue::BoxedValue(Box::new(String::from("payload")));

        assert_eq!(
            value.as_ref_value::<String>(),
            Some(&String::from("payload"))
        );
        assert_eq!(value.to_value::<String>(), Some(String::from("payload")));
        assert_eq!(
            value.as_own_value::<String>(),
            Some(String::from("payload"))
        );
    }

    #[test]
    fn number_enum_is_copy_and_comparable() {
        let a = Number::Int(1);
        let b = a; // Copy
        assert_eq!(a, b);
        assert_eq!(Number::Float(1.0), Number::Float(1.0));
        assert_ne!(Number::Int(1), Number::UInt(1));
    }
}
