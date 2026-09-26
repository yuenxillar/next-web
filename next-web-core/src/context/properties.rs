use dyn_clone::DynClone;

use std::error::Error;

use crate::env::{BindError, ConfigurableEnvironment};
use crate::util::indexmap::IndexMap;

/// Configuration properties of an application.
///
/// The properties a type declares with `#[configuration_properties(prefix =
/// "next.messages")]` are bound from the [`ConfigurableEnvironment`] the
/// application runs in, so the environment variables of the process, the
/// arguments of the command line and the placeholders of a value take part in
/// the binding like the properties of a property file do.
///
/// The generated implementation binds the properties with a [`Binder`] and
/// registers them in the [`ApplicationContext`](crate::ApplicationContext)
/// under [`name`](Self::name), which makes them available to the singletons
/// that depend on them.
pub trait ConfigurationProperties: DynClone + Send + Sync {
    /// Returns the name the properties are registered under.
    fn name(&self) -> &'static str;

    /// Returns the prefix the properties are bound from.
    fn prefix(&self) -> &'static str;

    /// Returns the properties the environment describes.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment the properties are bound from.
    ///
    /// # Errors
    ///
    /// Returns [`BindError`] when the properties cannot be bound.
    fn bind(environment: &dyn ConfigurableEnvironment) -> Result<Self, BindError>
    where
        Self: Sized;

    /// Binds the properties from the environment and registers them.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the properties are registered in.
    /// * `environment` - The environment the properties are bound from.
    ///
    /// # Errors
    ///
    /// Returns the error of the binding, or of the registration.
    fn register(
        &self,
        ctx: &mut dyn crate::ApplicationContext,
        environment: &dyn ConfigurableEnvironment,
    ) -> Result<(), Box<dyn Error>>;
}

dyn_clone::clone_trait_object!(ConfigurationProperties);

/// Flattens a value of the configuration file into the dotted names of a
/// property source.
///
/// The keys of a mapping are joined with dots, and the elements of a list are
/// indexed in brackets: `a: { b: [x, y] }` becomes `a.b[0] = x` and
/// `a.b[1] = y`.
///
/// # Arguments
///
/// * `value` - The value to flatten.
/// * `prefix` - The name of the value.
/// * `properties` - The properties that are collected.
fn flatten(value: &serde_yaml::Value, prefix: &str, properties: &mut IndexMap<String, String>) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let key = match key {
                    serde_yaml::Value::String(key) => key.clone(),
                    key => scalar(key),
                };
                let name = if prefix.is_empty() {
                    key
                } else {
                    format!("{prefix}.{key}")
                };

                flatten(value, &name, properties);
            }
        }
        serde_yaml::Value::Sequence(elements) => {
            for (index, value) in elements.iter().enumerate() {
                flatten(value, &format!("{prefix}[{index}]"), properties);
            }
        }
        value => {
            properties.insert(prefix.to_owned(), scalar(value));
        }
    }
}

/// Returns the text of a scalar value of the configuration file.
///
/// # Arguments
///
/// * `value` - The value to read.
fn scalar(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Null => String::new(),
        serde_yaml::Value::Bool(value) => value.to_string(),
        serde_yaml::Value::Number(value) => value.to_string(),
        serde_yaml::Value::String(value) => value.clone(),
        serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_)
        | serde_yaml::Value::Tagged(_) => String::new(),
    }
}

fn helper(temporary: Option<&serde_yaml::Value>, value: &mut serde_yaml::Value) {
    match value {
        serde_yaml::Value::String(s) => {
            let s = s.trim();
            if s.starts_with("${") && s.ends_with("}") {
                let key = s[2..s.len() - 1].to_string();

                if key
                    .as_str()
                    .chars()
                    .filter(|&c| c != '_' && c.is_alphabetic())
                    .all(|c| c.is_uppercase())
                {
                    // My suggestion is to panic directly
                    let var = match std::env::var(&key) {
                        Ok(var) => var,
                        Err(_) => panic!(
                            "In the configuration file, the environment variable [{}] cannot be obtained. Please check the environment configuration.",
                            key
                        ),
                    };
                    *value = serde_yaml::Value::String(var);
                    return;
                }
                let mut temporary = match temporary {
                    Some(temporary) => temporary,
                    None => return,
                };

                let mut iter = key.split('.').peekable();
                while let Some(parts) = iter.next() {
                    match temporary.get(parts) {
                        Some(val) => {
                            temporary = val;
                        }
                        None => panic!("The key [{}] is not found in the configuration file", key),
                    }

                    if iter.peek().is_none() {
                        // End of the key
                        *value = temporary.clone();
                    }
                }
            }
        }
        serde_yaml::Value::Mapping(mapping) => {
            mapping
                .iter_mut()
                .for_each(|(_, value)| helper(temporary, value));
        }
        _ => return,
    };
}
