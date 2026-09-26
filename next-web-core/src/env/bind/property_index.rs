//! The property names an environment exposes.

use std::collections::HashSet;

use crate::env::ConfigurableEnvironment;

/// The names of the properties that a [`ConfigurableEnvironment`] exposes.
///
/// Only the property sources that enumerate their names contribute to the
/// index. A source that cannot be enumerated is skipped; the properties it
/// resolves are resolved through the sources it adapts, which are enumerated.
pub(crate) struct PropertyIndex {
    names: Vec<String>,
}

impl PropertyIndex {
    /// Returns the names of the properties the environment exposes.
    ///
    /// The names are collected in search order, and a name that several
    /// property sources hold is only collected once.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment to collect the names from.
    pub(crate) fn of(environment: &dyn ConfigurableEnvironment) -> Self {
        let mut names = Vec::new();
        let mut collected = HashSet::new();

        for source in environment.property_sources_ref().iter() {
            // A stub source only reserves the position of a source that is
            // created later, so it holds no name to contribute.
            if source.is_stub() {
                continue;
            }

            for name in source.property_names() {
                if collected.insert(name.clone()) {
                    names.push(name);
                }
            }
        }

        Self { names }
    }

    /// Returns whether the environment holds a property below the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look the properties of up.
    pub(crate) fn has_descendants(&self, key: &str) -> bool {
        self.names
            .iter()
            .any(|name| relative_name(name, key).is_some())
    }

    /// Returns the properties below the given key.
    ///
    /// Each property is returned as its path relative to the key, together
    /// with its name, so that the tree of the properties can be built by
    /// splitting the paths.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look the properties of up.
    pub(crate) fn descendants(&self, key: &str) -> Vec<(&str, &str)> {
        self.names
            .iter()
            .filter_map(|name| relative_name(name, key).map(|relative| (relative, name.as_str())))
            .collect()
    }
}

/// Returns the name of the given property relative to the given key, or `None`
/// when the property is not below the key.
///
/// A key is empty when the properties are bound from the root of the
/// environment, in which case every property is below the key.
///
/// The name of an environment variable continues its key with an underscore
/// instead of a dot, so the properties below the key `NEXT_LOGGING_FILE` are
/// the properties `NEXT_LOGGING_FILE_NAME` and `NEXT_LOGGING_FILE_PATH`. Only
/// the name of an environment variable is read that way; the underscore of a
/// dotted property does not nest it.
///
/// A key that is written with dots is bound from the name of an environment
/// variable as well, so the properties below the key `next.logging.levels` are
/// the environment variables `NEXT_LOGGING_LEVELS_*`.
///
/// # Arguments
///
/// * `name` - The name of the property.
/// * `key` - The key the property has to be below.
fn relative_name<'a>(name: &'a str, key: &str) -> Option<&'a str> {
    if key.is_empty() {
        return Some(name);
    }

    if let Some(rest) = name.strip_prefix(key) {
        return match rest.chars().next()? {
            // A nested property, the key of which is a prefix of its name.
            '.' => Some(&rest[1..]),
            // The element of a list, `key[0]` for example.
            '[' => Some(rest),
            // The name of a nested environment variable, `NEXT_LOGGING_FILE_PATH`
            // below the key `NEXT_LOGGING_FILE` for example.
            '_' if is_environment_name(key) => Some(&rest[1..]),
            _ => None,
        };
    }

    if is_environment_name(key) {
        return None;
    }

    // The name of an environment variable is the name of the property it holds
    // uppercased with its dots replaced by underscores.
    name.strip_prefix(&environment_name(key))?
        .strip_prefix('_')
}

/// Returns whether the given key is the name of an environment variable, which
/// is an uppercased name the parts of which are joined with underscores.
///
/// # Arguments
///
/// * `key` - The key to test.
fn is_environment_name(key: &str) -> bool {
    key.chars().all(|character| {
        character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
    })
}

/// Returns the names a field of the given parent may be bound from, in the
/// order they are tried.
///
/// The name of the field is tried as it is, which is how the properties of an
/// application are written, then with the underscores of the field replaced by
/// dashes, and finally as an environment variable. An environment variable name
/// is uppercased, and the dots of a property name are replaced by underscores,
/// so that the field `base_name` of the `next.messages` parent is bound from
/// `NEXT_MESSAGES_BASE_NAME` or `NEXT_MESSAGES_BASENAME`.
///
/// # Arguments
///
/// * `parent` - The key the field is below.
/// * `field` - The name of the field.
pub(crate) fn candidate_names(parent: &str, field: &str) -> Vec<String> {
    let exact = join(parent, field);
    let dashed = join(parent, &field.replace('_', "-"));

    let mut candidates = vec![exact.clone()];

    if dashed != exact {
        candidates.push(dashed.clone());
    }

    let environment = environment_name(&exact);
    candidates.push(environment);

    let environment_of_dashed = environment_name(&dashed).replace('-', "");
    if !candidates.contains(&environment_of_dashed) {
        candidates.push(environment_of_dashed);
    }

    candidates
}

/// Joins the key of a parent and the name of a field with a dot.
///
/// # Arguments
///
/// * `parent` - The key of the parent, which may be empty.
/// * `field` - The name of the field.
fn join(parent: &str, field: &str) -> String {
    if parent.is_empty() {
        field.to_owned()
    } else {
        format!("{parent}.{field}")
    }
}

/// Returns the name of the environment variable a property is bound from.
///
/// # Arguments
///
/// * `name` - The name of the property.
fn environment_name(name: &str) -> String {
    name.to_ascii_uppercase().replace('.', "_")
}

#[cfg(test)]
mod tests {
    use crate::env::{BaseEnvironment, ConfigurableEnvironment, MapPropertySource};
    use crate::util::indexmap::IndexMap;

    use super::*;

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
    fn collects_the_names_of_the_property_sources() {
        let environment = environment(&[
            ("next.messages.base_name", "messages"),
            ("next.messages.cache_duration", "30"),
        ]);

        let index = PropertyIndex::of(&environment);

        assert!(index.has_descendants("next.messages"));
        assert!(!index.has_descendants("next.datasource"));
    }

    #[test]
    fn does_not_treat_a_name_with_a_common_prefix_as_nested() {
        let environment = environment(&[("next.messages2.base_name", "messages")]);

        assert!(!PropertyIndex::of(&environment).has_descendants("next.messages"));
    }

    #[test]
    fn treats_the_name_of_an_environment_variable_as_a_parent() {
        let environment = environment(&[
            ("NEXT_LOGGING_FILE_NAME", "next.log"),
            ("NEXT_LOGGING_FILE_PATH", "./logs"),
        ]);

        assert!(PropertyIndex::of(&environment).has_descendants("NEXT_LOGGING_FILE"));
    }

    #[test]
    fn does_not_nest_a_dotted_property_with_an_underscore() {
        let environment = environment(&[("next.messages_extra.base_name", "messages")]);

        assert!(!PropertyIndex::of(&environment).has_descendants("next.messages"));
    }

    #[test]
    fn treats_the_name_of_an_environment_variable_as_a_dotted_key() {
        let environment = environment(&[
            ("NEXT_LOGGING_LEVELS_SQLX", "debug"),
            ("NEXT_LOGGING_LEVELS_NEXT_WEB", "trace"),
        ]);

        let index = PropertyIndex::of(&environment);

        assert!(index.has_descendants("next.logging.levels"));
        assert_eq!(
            index.descendants("next.logging.levels"),
            vec![
                ("SQLX", "NEXT_LOGGING_LEVELS_SQLX"),
                ("NEXT_WEB", "NEXT_LOGGING_LEVELS_NEXT_WEB")
            ]
        );
    }

    #[test]
    fn looks_the_properties_of_a_list_up() {
        let environment = environment(&[
            ("next.messages.common[0]", "first"),
            ("next.messages.common[1]", "second"),
        ]);

        let index = PropertyIndex::of(&environment);
        let descendants = index.descendants("next.messages");

        assert_eq!(
            descendants,
            vec![
                ("common[0]", "next.messages.common[0]"),
                ("common[1]", "next.messages.common[1]")
            ]
        );
    }

    #[test]
    fn binds_the_root_when_the_key_is_empty() {
        let environment = environment(&[("base_name", "messages")]);

        assert_eq!(
            PropertyIndex::of(&environment).descendants(""),
            vec![("base_name", "base_name")]
        );
    }

    #[test]
    fn tries_the_relaxed_names_of_a_field() {
        assert_eq!(
            candidate_names("next.messages", "base_name"),
            vec![
                "next.messages.base_name",
                "next.messages.base-name",
                "NEXT_MESSAGES_BASE_NAME",
                "NEXT_MESSAGES_BASENAME"
            ]
        );
    }

    #[test]
    fn tries_the_names_of_a_field_of_the_root() {
        assert_eq!(candidate_names("", "port"), vec!["port", "PORT"]);
    }
}
