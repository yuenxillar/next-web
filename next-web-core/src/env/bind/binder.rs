//! Binding the properties of an environment to a type.

use serde::de::DeserializeOwned;

use std::collections::HashMap;

use crate::env::ConfigurableEnvironment;

use super::BindError;
use super::property_deserializer::PropertyDeserializer;
use super::property_index::PropertyIndex;
use super::property_tree::{PropertyNode, PropertyTreeDeserializer};

/// Binds the properties of a [`ConfigurableEnvironment`] to a type.
///
/// The properties below the prefix of the binder are bound to the target, so
/// the value of the property `next.messages.base_name` is the value of the
/// field `base_name` of a target that is bound from the `next.messages` prefix.
/// The properties are resolved by the environment, which searches its property
/// sources in order and replaces the placeholders of the values it resolves, so
/// the environment variables of the process and the arguments of the command
/// line take part in the binding like the properties of a property file do.
///
/// # Examples
///
/// ```rust
/// use std::collections::HashMap;
///
/// use next_web_core::env::bind::Binder;
/// use next_web_core::env::{ConfigurableEnvironment, MapPropertySource, StandardEnvironment};
/// use next_web_core::util::indexmap::IndexMap;
///
/// let mut environment = StandardEnvironment::new();
/// environment
///     .property_sources()
///     .add_first(Box::new(MapPropertySource::new(
///         "test".to_owned(),
///         IndexMap::from([(
///             "next.messages.base_name".to_owned(),
///             "messages".to_owned(),
///         )]),
///     )));
///
/// let messages: HashMap<String, String> =
///     Binder::new(&environment, "next.messages").bind().unwrap();
///
/// assert_eq!(
///     messages.get("base_name").map(String::as_str),
///     Some("messages")
/// );
/// ```
pub struct Binder<'a> {
    environment: &'a dyn ConfigurableEnvironment,
    prefix: String,
}

impl<'a> Binder<'a> {
    /// Creates a binder of the properties below the given prefix.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to read the properties from.
    /// * `prefix` - The prefix the properties of the target are below. An empty
    ///   prefix binds the properties of the root of the environment.
    pub fn new(environment: &'a dyn ConfigurableEnvironment, prefix: impl Into<String>) -> Self {
        Self {
            environment,
            prefix: prefix.into(),
        }
    }

    /// Returns the prefix the properties of the target are below.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Returns the value the properties below the prefix describe.
    ///
    /// A field the environment holds no property for is not set, so a required
    /// field, which is a field that is neither an [`Option`] nor a field with a
    /// `#[serde(default)]` attribute, is reported as missing.
    ///
    /// # Errors
    ///
    /// Returns [`BindError`] when the properties cannot be bound to `T`,
    /// because a property that is required is absent or because the value of a
    /// property cannot be read as the type of its field.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use next_web_core::env::bind::Binder;
    /// use next_web_core::env::{ConfigurableEnvironment, MapPropertySource, StandardEnvironment};
    /// use next_web_core::util::indexmap::IndexMap;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Messages {
    ///     base_name: Option<String>,
    ///     cache_duration: Option<u64>,
    /// }
    ///
    /// let mut environment = StandardEnvironment::new();
    /// environment
    ///     .property_sources()
    ///     .add_first(Box::new(MapPropertySource::new(
    ///         "test".to_owned(),
    ///         IndexMap::from([(
    ///             "next.messages.cache_duration".to_owned(),
    ///             "30".to_owned(),
    ///         )]),
    ///     )));
    ///
    /// let messages: Messages = Binder::new(&environment, "next.messages")
    ///     .bind()
    ///     .unwrap();
    ///
    /// assert_eq!(messages.base_name, None);
    /// assert_eq!(messages.cache_duration, Some(30));
    /// ```
    pub fn bind<T>(&self) -> Result<T, BindError>
    where
        T: DeserializeOwned,
    {
        let index = PropertyIndex::of(self.environment);
        let deserializer = PropertyDeserializer::new(self.environment, &index, self.prefix.clone());

        serde_path_to_error::deserialize(deserializer).map_err(|error| {
            let path = error.path().to_string();

            error.into_inner().with_path(path)
        })
    }

    /// Returns the properties below the prefix, grouped by the keys that hold
    /// them.
    ///
    /// Each key below the prefix is bound to `T` from the properties below it,
    /// and a key that holds nothing `T` can be bound from is skipped. The
    /// properties of `T` itself are therefore not bound again, and a key that
    /// holds the value of a property of `T` is skipped as well.
    ///
    /// Returns `None` when the prefix holds no property at all.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    ///
    /// use next_web_core::env::bind::Binder;
    /// use next_web_core::env::{ConfigurableEnvironment, MapPropertySource, StandardEnvironment};
    /// use next_web_core::util::indexmap::IndexMap;
    ///
    /// #[derive(serde::Deserialize)]
    /// struct Client {
    ///     host: String,
    /// }
    ///
    /// let mut environment = StandardEnvironment::new();
    /// environment
    ///     .property_sources()
    ///     .add_first(Box::new(MapPropertySource::new(
    ///         "test".to_owned(),
    ///         IndexMap::from([
    ///             ("next.clients.first.host".to_owned(), "127.0.0.1".to_owned()),
    ///             ("next.clients.second.host".to_owned(), "127.0.0.2".to_owned()),
    ///         ]),
    ///     )));
    ///
    /// let clients: HashMap<String, Client> = Binder::new(&environment, "next.clients")
    ///     .bind_dynamic()
    ///     .unwrap();
    ///
    /// assert_eq!(clients.len(), 2);
    /// ```
    pub fn bind_dynamic<T>(&self) -> Option<HashMap<String, T>>
    where
        T: DeserializeOwned,
    {
        let index = PropertyIndex::of(self.environment);
        let node = PropertyNode::of(self.environment, &index, &self.prefix);

        let PropertyNode::Map(properties) = node else {
            return None;
        };

        if properties.is_empty() {
            return None;
        }

        Some(
            properties
                .into_iter()
                .filter_map(|(key, node)| {
                    T::deserialize(PropertyTreeDeserializer::new(node))
                        .ok()
                        .map(|value| (key, value))
                })
                .collect(),
        )
    }
}

/// Binds the properties of an environment to a type.
///
/// This is the convenience of [`Binder`] that can be called on the trait object
/// of an environment, where a generic method could not be declared on
/// [`ConfigurableEnvironment`] itself, since a trait that is used as a trait
/// object cannot hold a method with type parameters of its own.
///
/// # Examples
///
/// ```rust
/// use std::collections::HashMap;
///
/// use next_web_core::env::bind::ConfigurableEnvironmentBindExt;
/// use next_web_core::env::{ConfigurableEnvironment, MapPropertySource, StandardEnvironment};
/// use next_web_core::util::indexmap::IndexMap;
///
/// let mut environment = StandardEnvironment::new();
/// environment
///     .property_sources()
///     .add_first(Box::new(MapPropertySource::new(
///         "test".to_owned(),
///         IndexMap::from([(
///             "next.messages.base_name".to_owned(),
///             "messages".to_owned(),
///         )]),
///     )));
///
/// let messages: HashMap<String, String> = environment.bind("next.messages").unwrap();
///
/// assert_eq!(
///     messages.get("base_name").map(String::as_str),
///     Some("messages")
/// );
/// ```
pub trait ConfigurableEnvironmentBindExt {
    /// Returns the value the properties below the given prefix describe.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix the properties of the target are below.
    ///
    /// # Errors
    ///
    /// Returns [`BindError`] when the properties cannot be bound to `T`.
    fn bind<T>(&self, prefix: &str) -> Result<T, BindError>
    where
        T: DeserializeOwned;
}

impl ConfigurableEnvironmentBindExt for dyn ConfigurableEnvironment {
    fn bind<T>(&self, prefix: &str) -> Result<T, BindError>
    where
        T: DeserializeOwned,
    {
        Binder::new(self, prefix).bind()
    }
}

impl<E> ConfigurableEnvironmentBindExt for E
where
    E: ConfigurableEnvironment,
{
    fn bind<T>(&self, prefix: &str) -> Result<T, BindError>
    where
        T: DeserializeOwned,
    {
        Binder::new(self, prefix).bind()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::env::{BaseEnvironment, ConfigurableEnvironment, MapPropertySource};
    use crate::util::indexmap::IndexMap;
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Messages {
        base_name: Option<String>,
        cache_duration: Option<u64>,
    }

    /// Creates an environment holding the given properties.
    fn environment(properties: &[(&str, &str)]) -> BaseEnvironment {
        let mut environment = BaseEnvironment::new();
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect::<IndexMap<_, _>>(),
            )));

        environment
    }

    #[test]
    fn binds_the_properties_below_the_prefix() {
        let environment = environment(&[
            ("next.messages.base_name", "messages"),
            ("next.messages.cache_duration", "30"),
            ("next.datasource.url", "jdbc:h2:mem:test"),
        ]);

        let messages: Messages = Binder::new(&environment, "next.messages").bind().unwrap();

        assert_eq!(
            messages,
            Messages {
                base_name: Some("messages".to_owned()),
                cache_duration: Some(30),
            }
        );
    }

    #[test]
    fn binds_the_properties_of_the_root_when_the_prefix_is_empty() {
        #[derive(Debug, Deserialize)]
        struct Properties {
            name: String,
        }

        let environment = environment(&[("name", "demo")]);
        let properties: Properties = Binder::new(&environment, "").bind().unwrap();

        assert_eq!(properties.name, "demo");
    }

    #[test]
    fn binds_the_properties_of_a_trait_object() {
        let environment = environment(&[("next.messages.base_name", "messages")]);
        let environment: &dyn ConfigurableEnvironment = &environment;

        let messages: Messages = Binder::new(environment, "next.messages").bind().unwrap();

        assert_eq!(messages.base_name.as_deref(), Some("messages"));
    }

    #[test]
    fn binds_the_properties_with_the_convenience_of_the_environment() {
        let environment = environment(&[("next.messages.base_name", "messages")]);

        let messages: Messages = environment.bind("next.messages").unwrap();

        assert_eq!(messages.base_name.as_deref(), Some("messages"));
    }

    #[test]
    fn binds_the_properties_of_the_trait_object_of_an_environment() {
        let environment = environment(&[("next.messages.base_name", "messages")]);
        let environment: &dyn ConfigurableEnvironment = &environment;

        let messages: Messages = environment.bind("next.messages").unwrap();

        assert_eq!(messages.base_name.as_deref(), Some("messages"));
    }

    #[test]
    fn binds_the_properties_of_a_shared_environment() {
        let environment: Arc<dyn ConfigurableEnvironment> =
            Arc::new(environment(&[("next.messages.base_name", "messages")]));

        let messages: Messages = environment.bind("next.messages").unwrap();

        assert_eq!(messages.base_name.as_deref(), Some("messages"));
    }

    #[test]
    fn reports_the_prefix_and_the_fields_of_a_failed_binding() {
        let environment = environment(&[("next.messages.cache_duration", "forever")]);
        let binder = Binder::new(&environment, "next.messages");

        let error = binder.bind::<Messages>().unwrap_err();

        assert_eq!(binder.prefix(), "next.messages");
        assert_eq!(error.path(), Some("cache_duration"));
        assert_eq!(
            error.to_string(),
            format!("failed to bind `cache_duration`: {}", error.message())
        );
    }
}
