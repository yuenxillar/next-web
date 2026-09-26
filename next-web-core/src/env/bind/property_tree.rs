//! The properties of an environment below a key, as a tree of values.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

use crate::env::ConfigurableEnvironment;
use crate::util::indexmap::IndexMap;

use super::property_index::PropertyIndex;
use super::{BindError, PropertyValueDeserializer};

/// Deserializes a tree of properties by handing the node of the tree to the
/// visitor that matches it.
///
/// A value is deserialized as the property value it holds, the properties of a
/// key are deserialized as a map, and the elements of a list are deserialized
/// as a sequence.
macro_rules! deserialize_tree {
    ($($method:ident $(, $argument:ident : $ty:ty)* ;)*) => {
        $(
            fn $method<V>(self, $($argument: $ty,)* visitor: V) -> Result<V::Value, BindError>
            where
                V: Visitor<'de>,
            {
                match self.node {
                    PropertyNode::Value(value) => de::Deserializer::$method(
                        PropertyValueDeserializer::new(value),
                        $($argument,)*
                        visitor,
                    ),
                    PropertyNode::Map(map) => visitor.visit_map(MapNodeAccess::new(map)),
                    PropertyNode::Seq(elements) => {
                        visitor.visit_seq(SeqNodeAccess::new(elements))
                    }
                }
            }
        )*
    };
}

/// A property of an environment, and the properties below it.
pub(crate) enum PropertyNode {
    /// A single value.
    Value(String),

    /// The properties below a key, in the order they are found.
    Map(IndexMap<String, PropertyNode>),

    /// The elements of a list.
    Seq(Vec<PropertyNode>),
}

impl PropertyNode {
    /// Builds the tree of the properties of the environment below the given
    /// key.
    ///
    /// The values of the tree are the values the environment resolves, so the
    /// placeholders of a value are replaced by the properties they name.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to read the properties from.
    /// * `index` - The names of the properties the environment exposes.
    /// * `key` - The key the tree is built for.
    pub(crate) fn of(
        environment: &dyn ConfigurableEnvironment,
        index: &PropertyIndex,
        key: &str,
    ) -> Self {
        let mut node = PropertyNode::Map(IndexMap::new());

        for (relative, name) in index.descendants(key) {
            let Some(value) = environment.get_property(name) else {
                continue;
            };

            insert(&mut node, &path_of(relative), value);
        }

        node
    }
}

/// A step of the path of a property, relative to the key of a tree.
enum Step {
    /// An element of a mapping.
    Key(String),

    /// An element of a list.
    Index(usize),
}

/// Returns the steps that lead to the value of a property.
///
/// The path of a property holds the keys of the mappings it is nested in,
/// separated by dots, and the indices of the lists it is an element of, in
/// brackets: `common[0]` is the first element of the list `common`.
///
/// # Arguments
///
/// * `relative` - The path of the property relative to the key of the tree.
fn path_of(relative: &str) -> Vec<Step> {
    let mut steps = Vec::new();

    for element in relative.split('.') {
        let mut rest = element;

        if let Some(bracket) = rest.find('[') {
            let (key, indices) = rest.split_at(bracket);

            if !key.is_empty() {
                steps.push(Step::Key(key.to_owned()));
            }

            rest = indices;
        }

        while let Some(index) = rest.strip_prefix('[') {
            let Some(end) = index.find(']') else {
                break;
            };

            if let Ok(position) = index[..end].parse::<usize>() {
                steps.push(Step::Index(position));
            }

            rest = &index[end + 1..];
        }

        if !rest.is_empty() {
            steps.push(Step::Key(rest.to_owned()));
        }
    }

    steps
}

/// Inserts a value into the tree at the given path.
///
/// A key that leads to a value is turned into a map, and an index that leads to
/// a value is turned into a list, so that the shape of the tree follows the
/// shape of the path. A property that is both a value and a key of the tree
/// keeps the properties below it.
///
/// # Arguments
///
/// * `node` - The node to insert the value into.
/// * `steps` - The steps of the path that lead to the value.
/// * `value` - The value to insert.
fn insert(node: &mut PropertyNode, steps: &[Step], value: String) {
    match steps.split_first() {
        None => *node = PropertyNode::Value(value),
        Some((Step::Key(key), rest)) => {
            if !matches!(node, PropertyNode::Map(_)) {
                *node = PropertyNode::Map(IndexMap::new());
            }

            let PropertyNode::Map(map) = node else {
                return;
            };

            let child = map
                .entry(key.clone())
                .or_insert_with(|| PropertyNode::Map(IndexMap::new()));

            insert(child, rest, value);
        }
        Some((Step::Index(index), rest)) => {
            if !matches!(node, PropertyNode::Seq(_)) {
                *node = PropertyNode::Seq(Vec::new());
            }

            let PropertyNode::Seq(elements) = node else {
                return;
            };

            while elements.len() <= *index {
                elements.push(PropertyNode::Map(IndexMap::new()));
            }

            insert(&mut elements[*index], rest, value);
        }
    }
}

/// A deserializer for a tree of properties.
pub(crate) struct PropertyTreeDeserializer {
    node: PropertyNode,
}

impl PropertyTreeDeserializer {
    /// Creates a deserializer for the given tree.
    ///
    /// # Arguments
    ///
    /// * `node` - The tree to deserialize.
    pub(crate) fn new(node: PropertyNode) -> Self {
        Self { node }
    }
}

impl<'de> de::Deserializer<'de> for PropertyTreeDeserializer {
    type Error = BindError;

    deserialize_tree! {
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
        deserialize_map;
        deserialize_struct, _name: &'static str, _fields: &'static [&'static str];
        deserialize_enum, _name: &'static str, _variants: &'static [&'static str];
        deserialize_identifier;
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        match &self.node {
            // The key of a property that holds nothing does not describe a
            // value of the target, so the target is not set.
            PropertyNode::Map(map) if map.is_empty() => visitor.visit_none(),
            PropertyNode::Seq(elements) if elements.is_empty() => visitor.visit_none(),
            _ => visitor.visit_some(self),
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

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, BindError>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// The access to the properties of a key of a tree.
struct MapNodeAccess {
    properties: std::vec::IntoIter<(String, PropertyNode)>,
    value: Option<PropertyNode>,
}

impl MapNodeAccess {
    /// Creates an access to the properties of the given map.
    ///
    /// # Arguments
    ///
    /// * `properties` - The properties of the map.
    fn new(properties: IndexMap<String, PropertyNode>) -> Self {
        Self {
            properties: properties.into_iter().collect::<Vec<_>>().into_iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapNodeAccess {
    type Error = BindError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, BindError>
    where
        K: DeserializeSeed<'de>,
    {
        match self.properties.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(PropertyValueDeserializer::new(key))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<K>(&mut self, seed: K) -> Result<K::Value, BindError>
    where
        K: DeserializeSeed<'de>,
    {
        let value = self.value.take().ok_or_else(|| {
            BindError::new("the value of a property was asked for before its key")
        })?;

        seed.deserialize(PropertyTreeDeserializer::new(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.properties.len())
    }
}

/// The access to the elements of a list of a tree.
struct SeqNodeAccess {
    elements: std::vec::IntoIter<PropertyNode>,
}

impl SeqNodeAccess {
    /// Creates an access to the given elements.
    ///
    /// # Arguments
    ///
    /// * `elements` - The elements of the list.
    fn new(elements: Vec<PropertyNode>) -> Self {
        Self {
            elements: elements.into_iter(),
        }
    }
}

impl<'de> SeqAccess<'de> for SeqNodeAccess {
    type Error = BindError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, BindError>
    where
        T: DeserializeSeed<'de>,
    {
        match self.elements.next() {
            Some(element) => seed
                .deserialize(PropertyTreeDeserializer::new(element))
                .map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.elements.len())
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    /// Returns the tree of the properties of the given key.
    fn tree(properties: &[(&str, &str)], key: &str) -> PropertyNode {
        let mut environment = crate::env::BaseEnvironment::new();
        environment
            .property_sources()
            .add_last(Box::new(crate::env::MapPropertySource::new(
                "test".to_owned(),
                properties
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect::<IndexMap<_, _>>(),
            )));

        let index = PropertyIndex::of(&environment);

        PropertyNode::of(&environment, &index, key)
    }

    #[test]
    fn splits_the_path_of_a_property() {
        let steps: Vec<String> = path_of("common[1].name")
            .into_iter()
            .map(|step| match step {
                Step::Key(key) => key,
                Step::Index(index) => index.to_string(),
            })
            .collect();

        assert_eq!(steps, vec!["common", "1", "name"]);
    }

    #[test]
    fn builds_a_list_from_the_indices_of_a_path() {
        let node = tree(
            &[
                ("next.messages.common[0]", "first"),
                ("next.messages.common[1]", "second"),
            ],
            "next.messages",
        );

        let PropertyNode::Map(map) = node else {
            panic!("the tree of a key is a map");
        };
        let Some(PropertyNode::Seq(elements)) = map.get("common") else {
            panic!("the indices of a path build a list");
        };

        let values: Vec<&str> = elements
            .iter()
            .map(|element| match element {
                PropertyNode::Value(value) => value.as_str(),
                _ => panic!("the path leads to a value"),
            })
            .collect();

        assert_eq!(values, vec!["first", "second"]);
    }

    #[test]
    fn builds_the_properties_of_a_key() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Messages {
            base_name: String,
            common: Vec<String>,
        }

        let node = tree(
            &[
                ("next.messages.base_name", "messages"),
                ("next.messages.common[0]", "first"),
                ("next.messages.common[1]", "second"),
            ],
            "next.messages",
        );

        let messages = Messages::deserialize(PropertyTreeDeserializer::new(node)).unwrap();

        assert_eq!(
            messages,
            Messages {
                base_name: "messages".to_owned(),
                common: vec!["first".to_owned(), "second".to_owned()],
            }
        );
    }

    #[test]
    fn builds_a_map_from_the_properties_of_a_key() {
        let node = tree(&[("next.messages.base_name", "messages")], "next.messages");

        let messages = std::collections::HashMap::<String, String>::deserialize(
            PropertyTreeDeserializer::new(node),
        )
        .unwrap();

        assert_eq!(
            messages.get("base_name").map(String::as_str),
            Some("messages")
        );
    }

    #[test]
    fn builds_an_empty_map_from_a_key_that_holds_nothing() {
        let node = tree(
            &[("next.messages.base_name", "messages")],
            "next.datasource",
        );

        let properties = std::collections::HashMap::<String, String>::deserialize(
            PropertyTreeDeserializer::new(node),
        )
        .unwrap();

        assert!(properties.is_empty());
    }
}
