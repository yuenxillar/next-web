//! Deserializing a single property value.

use serde::de::{self, DeserializeSeed, EnumAccess, SeqAccess, Unexpected, VariantAccess, Visitor};

use super::BindError;

/// Deserializes the value of a property as a number, whichever width the target
/// asks for.
///
/// A value that cannot be read as a number of the type that is asked for is
/// reported as the wrong value.
macro_rules! deserialize_number {
    ($($method:ident($type:ty) => $visit:ident),* $(,)?) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, BindError>
            where
                V: Visitor<'de>,
            {
                match self.value.trim().parse::<$type>() {
                    Ok(value) => visitor.$visit(value),
                    Err(_) => Err(invalid_value(&self.value, &visitor)),
                }
            }
        )*
    };
}

/// A deserializer for the value of a single property.
///
/// The property sources of an environment hold their values as strings, so the
/// deserializer converts the value into the type the target asks for:
///
/// - A boolean accepts `true`/`false`, `on`/`off`, `yes`/`no` and `1`/`0`.
/// - A number is parsed from the value.
/// - A list is split on commas, which is how a list is written in a property
///   that cannot hold one.
/// - An enumeration is read as the name of a unit variant.
///
/// Any other type is reported as an unexpected string, which is the type the
/// property sources hold.
#[derive(Debug, Clone)]
pub(crate) struct PropertyValueDeserializer {
    value: String,
}

impl PropertyValueDeserializer {
    /// Creates a deserializer for the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to deserialize.
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Returns the elements the value of a list holds.
    fn elements(&self) -> std::vec::IntoIter<String> {
        elements_of(&self.value).into_iter()
    }
}

impl<'de> de::Deserializer<'de> for PropertyValueDeserializer {
    type Error = BindError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.value)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        match parse_bool(&self.value) {
            Some(value) => visitor.visit_bool(value),
            None => Err(invalid_type(&self.value, &visitor)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        let mut characters = self.value.trim().chars();

        match (characters.next(), characters.next()) {
            (Some(value), None) => visitor.visit_char(value),
            _ => Err(invalid_value(&self.value, &visitor)),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.value)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.value)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.value.into_bytes())
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.value.into_bytes())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        if self.value.trim().is_empty() {
            visitor.visit_unit()
        } else {
            Err(invalid_type(&self.value, &visitor))
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(ValueSeqAccess {
            elements: self.elements(),
        })
    }

    fn deserialize_tuple<V>(self, _length: usize, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _length: usize,
        visitor: V,
    ) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        Err(invalid_type(&self.value, &visitor))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        Err(invalid_type(&self.value, &visitor))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(UnitVariantAccess {
            variant: self.value,
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.value)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    deserialize_number! {
        deserialize_i8(i8) => visit_i8,
        deserialize_i16(i16) => visit_i16,
        deserialize_i32(i32) => visit_i32,
        deserialize_i64(i64) => visit_i64,
        deserialize_i128(i128) => visit_i128,
        deserialize_u8(u8) => visit_u8,
        deserialize_u16(u16) => visit_u16,
        deserialize_u32(u32) => visit_u32,
        deserialize_u64(u64) => visit_u64,
        deserialize_u128(u128) => visit_u128,
        deserialize_f32(f32) => visit_f32,
        deserialize_f64(f64) => visit_f64,
    }
}

/// Returns the elements of a list, which are separated by commas.
///
/// An empty value holds no element, so that a property that is set to nothing
/// binds an empty list.
///
/// # Arguments
///
/// * `value` - The value to split.
fn elements_of(value: &str) -> Vec<String> {
    let value = value.trim();

    if value.is_empty() {
        return Vec::new();
    }

    value
        .split(',')
        .map(|element| element.trim().to_owned())
        .collect()
}

/// Returns the boolean the value holds, in the forms a property file and an
/// environment variable write it in.
///
/// # Arguments
///
/// * `value` - The value to read.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "on" | "yes" | "1" => Some(true),
        "false" | "off" | "no" | "0" => Some(false),
        _ => None,
    }
}

/// Returns the error reported for a value that is not of the type the visitor
/// expects.
///
/// # Arguments
///
/// * `value` - The value that cannot be read.
/// * `visitor` - The visitor that expects another type.
fn invalid_type<'de, E, V>(value: &str, visitor: &V) -> E
where
    E: de::Error,
    V: Visitor<'de>,
{
    E::invalid_type(Unexpected::Str(value), visitor)
}

/// Returns the error reported for a value that cannot be converted into the
/// type the visitor expects, even though it is of the expected kind.
///
/// # Arguments
///
/// * `value` - The value that cannot be converted.
/// * `visitor` - The visitor that expects another value.
fn invalid_value<'de, E, V>(value: &str, visitor: &V) -> E
where
    E: de::Error,
    V: Visitor<'de>,
{
    E::invalid_value(Unexpected::Str(value), visitor)
}

/// The elements of the value of a list.
struct ValueSeqAccess {
    elements: std::vec::IntoIter<String>,
}

impl<'de> SeqAccess<'de> for ValueSeqAccess {
    type Error = BindError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, BindError>
    where
        T: DeserializeSeed<'de>,
    {
        match self.elements.next() {
            Some(element) => seed
                .deserialize(PropertyValueDeserializer::new(element))
                .map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.elements.len())
    }
}

/// The access to the variant of an enumeration that holds no data.
struct UnitVariantAccess {
    variant: String,
}

impl<'de> EnumAccess<'de> for UnitVariantAccess {
    type Error = BindError;
    type Variant = UnitVariant;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), BindError>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(PropertyValueDeserializer::new(self.variant))?;

        Ok((variant, UnitVariant))
    }
}

/// The access to an enumeration variant that holds no data.
struct UnitVariant;

impl<'de> VariantAccess<'de> for UnitVariant {
    type Error = BindError;

    fn unit_variant(self) -> Result<(), BindError> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, BindError>
    where
        T: DeserializeSeed<'de>,
    {
        Err(BindError::new("a property only holds a unit variant"))
    }

    fn tuple_variant<V>(self, _length: usize, _visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        Err(BindError::new("a property only holds a unit variant"))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        Err(BindError::new("a property only holds a unit variant"))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    /// Reads the value of a property as the given type.
    fn read<T: for<'de> Deserialize<'de>>(value: &str) -> Result<T, BindError> {
        T::deserialize(PropertyValueDeserializer::new(value))
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    enum Mode {
        #[serde(rename = "console")]
        Console,
        #[serde(rename = "log")]
        Log,
    }

    #[test]
    fn reads_strings() {
        assert_eq!(read::<String>("messages").unwrap(), "messages");
        assert_eq!(read::<String>("").unwrap(), "");
    }

    #[test]
    fn reads_booleans() {
        for value in ["true", "TRUE", "on", "yes", "1", " true "] {
            assert!(read::<bool>(value).unwrap(), "'{value}' should be true");
        }

        for value in ["false", "off", "no", "0"] {
            assert!(!read::<bool>(value).unwrap(), "'{value}' should be false");
        }

        assert!(read::<bool>("sideways").is_err());
    }

    #[test]
    fn reads_numbers() {
        assert_eq!(read::<u64>(" 30 ").unwrap(), 30);
        assert_eq!(read::<i32>("-1").unwrap(), -1);
        assert_eq!(read::<f64>("1.5").unwrap(), 1.5);

        assert!(read::<u64>("-1").is_err());
        assert!(read::<u64>("thirty").is_err());
    }

    #[test]
    fn reads_lists_separated_by_commas() {
        assert_eq!(
            read::<Vec<String>>("messages, errors").unwrap(),
            vec!["messages".to_owned(), "errors".to_owned()]
        );
        assert_eq!(read::<Vec<u16>>("1, 2").unwrap(), vec![1, 2]);
        assert_eq!(read::<Vec<String>>("").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn reads_options() {
        assert_eq!(
            read::<Option<String>>("messages").unwrap().as_deref(),
            Some("messages")
        );
    }

    #[test]
    fn reads_unit_variants() {
        assert_eq!(read::<Mode>("log").unwrap(), Mode::Log);
    }

    #[test]
    fn reports_a_value_a_type_cannot_be_read_from() {
        let error = read::<Mode>("sideways").unwrap_err();

        assert!(
            error.to_string().contains("sideways"),
            "unexpected error: {error}"
        );
    }
}
