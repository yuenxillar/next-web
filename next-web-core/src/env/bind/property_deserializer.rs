//! Deserializing the properties of an environment below a key.

use serde::de::{self, DeserializeSeed, MapAccess, Visitor};

use crate::env::ConfigurableEnvironment;

use super::property_index::{candidate_names, PropertyIndex};
use super::property_tree::{PropertyNode, PropertyTreeDeserializer};
use super::{BindError, PropertyValueDeserializer};

/// Deserializes a property by reading its value, and falls back to the tree of
/// the properties below it when the key holds no value itself.
///
/// The tree is used for the targets that cannot be read from a single value,
/// such as a map, and for the values of a list that are written as the
/// properties of the key instead of as a comma separated value.
macro_rules! deserialize_value_or_tree {
    ($($method:ident $(, $argument:ident : $ty:ty)* ;)*) => {
        $(
            fn $method<V>(self, $($argument: $ty,)* visitor: V) -> Result<V::Value, BindError>
            where
                V: Visitor<'de>,
            {
                match self.value() {
                    Some(value) => de::Deserializer::$method(
                        PropertyValueDeserializer::new(value),
                        $($argument,)*
                        visitor,
                    ),
                    None => de::Deserializer::$method(self.tree(), $($argument,)* visitor),
                }
            }
        )*
    };
}

/// A deserializer for the properties of an environment below a key.
///
/// The deserializer reads the properties below its key from the environment, so
/// the value of the property `next.messages.base_name` is the value of the
/// field `base_name` of the `next.messages` key. A field that is a struct
/// itself is deserialized from the properties below its own key, recursively.
///
/// The field names of the target decide which properties are read, which is why
/// a property source that does not enumerate its names does not have to be
/// supported for a struct to be bound. The names the fields are bound from are
/// described by [`candidate_names`](super::property_index::candidate_names).
pub(crate) struct PropertyDeserializer<'a> {
    environment: &'a dyn ConfigurableEnvironment,
    index: &'a PropertyIndex,
    key: String,
}

impl<'a> PropertyDeserializer<'a> {
    /// Creates a deserializer for the properties below the given key.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to read the properties from.
    /// * `index` - The names of the properties the environment exposes.
    /// * `key` - The key the properties of the target are below.
    pub(crate) fn new(
        environment: &'a dyn ConfigurableEnvironment,
        index: &'a PropertyIndex,
        key: impl Into<String>,
    ) -> Self {
        Self {
            environment,
            index,
            key: key.into(),
        }
    }

    /// Returns the value of this key, when the environment resolves one.
    ///
    /// The placeholders of the value are resolved by the environment.
    fn value(&self) -> Option<String> {
        self.environment.get_property(&self.key)
    }

    /// Returns the tree of the properties below this key.
    fn tree(&self) -> PropertyTreeDeserializer {
        PropertyTreeDeserializer::new(PropertyNode::of(self.environment, self.index, &self.key))
    }

    /// Returns the deserializer of the property the given field is bound from.
    ///
    /// A field is bound from the first name that the environment resolves,
    /// see [`candidate_names`](super::property_index::candidate_names). A field
    /// that only has properties below its key is bound from those properties.
    ///
    /// # Arguments
    ///
    /// * `field` - The name of the field.
    fn field(&self, field: &str) -> Option<PropertyDeserializer<'a>> {
        candidate_names(&self.key, field)
            .into_iter()
            .find_map(|key| {
                let bound =
                    self.environment.contains_property(&key) || self.index.has_descendants(&key);

                bound.then(|| PropertyDeserializer::new(self.environment, self.index, key))
            })
    }
}

impl<'de, 'a> de::Deserializer<'de> for PropertyDeserializer<'a> {
    type Error = BindError;

    deserialize_value_or_tree! {
        deserialize_any;
        deserialize_bool;
        deserialize_i8;
        deserialize_i16;
        deserialize_i32;
        deserialize_i64;
        deserialize_i128;
        deserialize_u8;
        deserialize_u16;
        deserialize_u32;
        deserialize_u64;
        deserialize_u128;
        deserialize_f32;
        deserialize_f64;
        deserialize_char;
        deserialize_str;
        deserialize_string;
        deserialize_bytes;
        deserialize_byte_buf;
        deserialize_unit;
        deserialize_unit_struct, _name: &'static str;
        deserialize_seq;
        deserialize_tuple, _length: usize;
        deserialize_tuple_struct, _name: &'static str, _length: usize;
        deserialize_enum, _name: &'static str, _variants: &'static [&'static str];
        deserialize_identifier;
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        // A key that holds neither a value nor properties below it does not
        // describe a value of the target, so the target is not set.
        if self.value().is_some() || self.index.has_descendants(&self.key) {
            visitor.visit_some(self)
        } else {
            visitor.visit_none()
        }
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

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.tree(), visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        let bound: Vec<(&'static str, PropertyDeserializer<'a>)> = fields
            .iter()
            .filter_map(|field| self.field(field).map(|value| (*field, value)))
            .collect();

        // A key that holds neither a value nor a property the target has a
        // field for holds nothing, in which case the fields that are required
        // are reported as missing by the target itself.
        if bound.is_empty() {
            return match self.value() {
                Some(value) => de::Deserializer::deserialize_struct(
                    PropertyValueDeserializer::new(value),
                    name,
                    fields,
                    visitor,
                ),
                None => visitor.visit_map(FieldAccess::new(Vec::new())),
            };
        }

        visitor.visit_map(FieldAccess::new(bound))
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// The access to the fields of a struct, which are the properties the struct is
/// bound from.
struct FieldAccess<'a> {
    fields: std::vec::IntoIter<(&'static str, PropertyDeserializer<'a>)>,
    value: Option<PropertyDeserializer<'a>>,
}

impl<'a> FieldAccess<'a> {
    /// Creates an access to the given fields.
    ///
    /// # Arguments
    ///
    /// * `fields` - The fields of the struct, together with the properties they
    ///   are bound from.
    fn new(fields: Vec<(&'static str, PropertyDeserializer<'a>)>) -> Self {
        Self {
            fields: fields.into_iter(),
            value: None,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for FieldAccess<'a> {
    type Error = BindError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, BindError>
    where
        K: DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some((field, value)) => {
                self.value = Some(value);
                seed.deserialize(FieldNameDeserializer(field)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<K>(&mut self, seed: K) -> Result<K::Value, BindError>
    where
        K: DeserializeSeed<'de>,
    {
        let value = self
            .value
            .take()
            .ok_or_else(|| BindError::new("the value of a field was asked for before its name"))?;

        seed.deserialize(value)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len())
    }
}

/// A deserializer for the name of a field.
struct FieldNameDeserializer(&'static str);

impl<'de> de::Deserializer<'de> for FieldNameDeserializer {
    type Error = BindError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::env::{BaseEnvironment, ConfigurableEnvironment, MapPropertySource};
    use crate::util::indexmap::IndexMap;

    use super::*;

    /// Creates an environment holding the given property sources, in search
    /// order.
    fn environment(sources: &[&[(&str, &str)]]) -> BaseEnvironment {
        let mut environment = BaseEnvironment::new();

        for (index, properties) in sources.iter().enumerate() {
            environment
                .property_sources()
                .add_last(Box::new(MapPropertySource::new(
                    format!("test{index}"),
                    properties
                        .iter()
                        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                        .collect::<IndexMap<_, _>>(),
                )));
        }

        environment
    }

    /// Binds the properties of the environment below the given key.
    fn bind<T: for<'de> Deserialize<'de>>(
        sources: &[&[(&str, &str)]],
        key: &str,
    ) -> Result<T, BindError> {
        let environment = environment(sources);
        let index = PropertyIndex::of(&environment);

        serde_path_to_error::deserialize(PropertyDeserializer::new(&environment, &index, key))
            .map_err(|error| {
                let path = error.path().to_string();

                error.into_inner().with_path(path)
            })
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Messages {
        base_name: Option<String>,
        fallback_to_system_locale: Option<bool>,
        cache_duration: Option<u64>,
        common_messages: Option<Vec<String>>,
    }

    #[test]
    fn binds_the_fields_of_a_struct() {
        let messages = bind::<Messages>(
            &[&[
                ("next.messages.base_name", "messages, errors"),
                ("next.messages.fallback_to_system_locale", "false"),
                ("next.messages.cache_duration", "30"),
                ("next.messages.common_messages[0]", "first"),
                ("next.messages.common_messages[1]", "second"),
            ]],
            "next.messages",
        )
        .unwrap();

        assert_eq!(
            messages,
            Messages {
                base_name: Some("messages, errors".to_owned()),
                fallback_to_system_locale: Some(false),
                cache_duration: Some(30),
                common_messages: Some(vec!["first".to_owned(), "second".to_owned()]),
            }
        );
    }

    #[test]
    fn leaves_the_fields_that_are_absent_unset() {
        let messages = bind::<Messages>(&[&[]], "next.messages").unwrap();

        assert_eq!(
            messages,
            Messages {
                base_name: None,
                fallback_to_system_locale: None,
                cache_duration: None,
                common_messages: None,
            }
        );
    }

    #[test]
    fn binds_the_fields_of_a_struct_that_holds_nothing_when_the_key_is_absent() {
        let messages = bind::<Messages>(
            &[&[("next.datasource.url", "jdbc:h2:mem:test")]],
            "next.messages",
        )
        .unwrap();

        assert_eq!(messages.base_name, None);
    }

    #[test]
    fn binds_the_relaxed_names_of_a_field() {
        let messages = bind::<Messages>(
            &[&[("next.messages.base-name", "messages")]],
            "next.messages",
        )
        .unwrap();

        assert_eq!(messages.base_name.as_deref(), Some("messages"));
    }

    #[test]
    fn binds_the_environment_variable_name_of_a_field() {
        let messages = bind::<Messages>(
            &[&[("NEXT_MESSAGES_BASE_NAME", "messages")]],
            "next.messages",
        )
        .unwrap();

        assert_eq!(messages.base_name.as_deref(), Some("messages"));
    }

    #[test]
    fn binds_a_list_that_is_written_as_a_comma_separated_value() {
        let messages = bind::<Messages>(
            &[&[("next.messages.common_messages", "first, second")]],
            "next.messages",
        )
        .unwrap();

        assert_eq!(
            messages.common_messages,
            Some(vec!["first".to_owned(), "second".to_owned()])
        );
    }

    #[test]
    fn binds_the_value_of_the_first_property_source_that_holds_it() {
        let messages = bind::<Messages>(
            &[
                &[("next.messages.base_name", "first")],
                &[("next.messages.base_name", "second")],
            ],
            "next.messages",
        )
        .unwrap();

        assert_eq!(messages.base_name.as_deref(), Some("first"));
    }

    #[test]
    fn resolves_the_placeholders_of_a_value() {
        let messages = bind::<Messages>(
            &[&[
                ("next.application.name", "demo"),
                ("next.messages.base_name", "${next.application.name}"),
            ]],
            "next.messages",
        )
        .unwrap();

        assert_eq!(messages.base_name.as_deref(), Some("demo"));
    }

    #[test]
    fn binds_the_fields_of_a_nested_struct() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Datasource {
            url: String,
            pool: Pool,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Pool {
            size: u16,
            name: Option<String>,
        }

        let datasource = bind::<Datasource>(
            &[
                &[
                    ("next.datasource.url", "jdbc:h2:mem:test"),
                    ("next.datasource.pool.size", "10"),
                ],
                &[("next.datasource.pool.name", "main")],
            ],
            "next.datasource",
        )
        .unwrap();

        assert_eq!(
            datasource,
            Datasource {
                url: "jdbc:h2:mem:test".to_owned(),
                pool: Pool {
                    size: 10,
                    name: Some("main".to_owned()),
                },
            }
        );
    }

    #[test]
    fn binds_the_fields_of_a_nested_struct_from_environment_variable_names() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Logging {
            level: Option<String>,
            file: Option<FileProperties>,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct FileProperties {
            name: Option<String>,
            path: Option<String>,
        }

        let logging = bind::<Logging>(
            &[&[
                ("NEXT_LOGGING_LEVEL", "debug"),
                ("NEXT_LOGGING_FILE_NAME", "next.log"),
                ("NEXT_LOGGING_FILE_PATH", "./logs"),
            ]],
            "next.logging",
        )
        .unwrap();

        assert_eq!(logging.level.as_deref(), Some("debug"));
        assert_eq!(
            logging.file,
            Some(FileProperties {
                name: Some("next.log".to_owned()),
                path: Some("./logs".to_owned()),
            })
        );
    }

    #[test]
    fn binds_a_map_of_the_properties_below_a_key() {
        let properties = bind::<std::collections::HashMap<String, String>>(
            &[&[
                ("next.messages.base_name", "messages"),
                ("next.messages.cache_duration", "30"),
            ]],
            "next.messages",
        )
        .unwrap();

        assert_eq!(
            properties.get("base_name").map(String::as_str),
            Some("messages")
        );
        assert_eq!(
            properties.get("cache_duration").map(String::as_str),
            Some("30")
        );
    }

    #[test]
    fn binds_a_map_of_the_structs_below_a_key() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Client {
            host: String,
            port: Option<u16>,
        }

        let clients = bind::<std::collections::HashMap<String, Client>>(
            &[&[
                ("next.clients.first.host", "127.0.0.1"),
                ("next.clients.first.port", "6379"),
                ("next.clients.second.host", "127.0.0.2"),
            ]],
            "next.clients",
        )
        .unwrap();

        assert_eq!(
            clients.get("first"),
            Some(&Client {
                host: "127.0.0.1".to_owned(),
                port: Some(6379),
            })
        );
        assert_eq!(
            clients.get("second"),
            Some(&Client {
                host: "127.0.0.2".to_owned(),
                port: None,
            })
        );
    }

    #[test]
    fn reports_the_fields_that_are_required_but_absent() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Datasource {
            url: String,
        }

        let error = bind::<Datasource>(&[&[("next.datasource.name", "main")]], "next.datasource")
            .unwrap_err();

        assert!(
            error.to_string().contains("url"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn reports_the_values_that_cannot_be_read_as_a_field() {
        let error = bind::<Messages>(
            &[&[("next.messages.cache_duration", "forever")]],
            "next.messages",
        )
        .unwrap_err();

        assert_eq!(error.path(), Some("cache_duration"));
        assert!(
            error.to_string().contains("cache_duration"),
            "unexpected error: {error}"
        );
    }
}
