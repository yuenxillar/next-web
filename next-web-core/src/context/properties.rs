use dyn_clone::DynClone;

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Read;
use std::path;

use crate::constants::application_constants::APPLICATION_CONFIG;
use crate::context::application_args::ApplicationArgs;
use crate::context::application_resources::ResourceLoader;

use super::application_resources::ApplicationResources;
use super::next_properties::NextProperties;
use crate::AutoRegister;

/// ApplicationProperties trait
///
/// This trait is used to insert properties into the application.
///
/// Please implement this trait in your application.
///
pub trait Properties: DynClone + AutoRegister {}

dyn_clone::clone_trait_object!(Properties);

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApplicationProperties {
    /// This Properties is Mapping data from the configuration file
    next: NextProperties,

    /// Only for register that have not been deserialized
    #[serde(skip_deserializing)]
    mapping: Option<serde_yaml::Value>,
}

impl ApplicationProperties {
    pub fn next(&self) -> &NextProperties {
        &self.next
    }

    /// Get a single value from the mapping
    ///
    /// # Example
    ///
    /// ```rust
    /// use next_web::application::application_properties::ApplicationProperties;
    /// use hashbrown::HashMap;
    ///
    /// let mut props = ApplicationProperties::default();
    /// props.set_mapping(HashMap::from([("key1".to_string(), serde_yaml::Value::String("value1".to_string()))]));
    /// assert_eq!(props.get_value::<String>("key1"), Some("value1".to_string()));
    ///
    pub fn get_value<T: serde::de::DeserializeOwned>(&self, key: impl AsRef<str>) -> Option<T> {
        let key = key.as_ref();
        if key.is_empty() {
            return None;
        }

        if let Some(mapping) = self.mapping.as_ref() {
            let keys: Vec<&str> = key.split(".").collect::<Vec<_>>();
            let index = keys.len();

            if index == 1 {
                return mapping
                    .get(key)
                    .map(|val| serde_yaml::from_value::<T>(val.clone()).ok())
                    .unwrap_or_default();
            }

            if let Some(mut value) = mapping.get(keys[0]) {
                for (i, k) in keys.iter().enumerate().skip(1) {
                    match value.get(k) {
                        Some(val) => {
                            if i == index - 1 {
                                return serde_yaml::from_value::<T>(val.clone()).ok();
                            }
                            value = val;
                        }
                        None => return None,
                    }
                }
            };
        }
        None
    }

    pub fn get_dynamic_value<T: serde::de::DeserializeOwned>(
        &self,
        key: impl AsRef<str>,
    ) -> Option<HashMap<String, T>> {
        let key = key.as_ref();
        if key.is_empty() {
            return None;
        }

        if let Some(mapping) = self.mapping.as_ref() {
            // 查找key的动态值
            let keys = key.split(".").collect::<Vec<_>>();
            let index = keys.len();

            if index <= 1 {
                return None;
            }

            if let Some(mut value) = mapping.get(keys[0]) {
                for (i, k) in keys.iter().enumerate().skip(1) {
                    match value.get(k) {
                        Some(val) => {
                            if i == index - 1 {
                                match val.as_mapping() {
                                    Some(_mapping) => {
                                        return Some(
                                            _mapping
                                                .iter()
                                                .filter_map(|(k, v)| {
                                                    let value = match serde_yaml::from_value::<T>(
                                                        v.to_owned(),
                                                    )
                                                    .ok()
                                                    {
                                                        Some(val) => val,
                                                        None => return None,
                                                    };

                                                    let key = k
                                                        .as_str()
                                                        .map(ToString::to_string)
                                                        .unwrap_or(format!("dynamic{}", index));
                                                    Some((key, value))
                                                })
                                                .collect::<HashMap<_, _>>(),
                                        );
                                    }

                                    None => return None,
                                }
                            }
                            value = val;
                        }
                        None => return None,
                    }
                }
            };
        }
        None
    }

    /// Replace the placeholders in the properties.
    pub fn replace_placeholders(&mut self) {
        // Two situations
        // ${author.name}   ${MY_ENV_VAR}

        let temporary = self.mapping.clone();
        self.mapping.as_mut().map(|mapping| {
            let mapping = match mapping.as_mapping_mut() {
                Some(mapping) => mapping,
                None => return,
            };

            mapping
                .iter_mut()
                .map(|val| val.1)
                .for_each(|value| helper(temporary.as_ref(), value));
        });
    }

    pub fn set_mapping(&mut self, mapping: serde_yaml::Value) {
        self.mapping = Some(mapping);
    }

    pub fn mapping(&self) -> Option<&serde_yaml::Value> {
        self.mapping.as_ref()
    }

    pub fn mapping_mut(&mut self) -> Option<&mut serde_yaml::Value> {
        self.mapping.as_mut()
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

fn into_application_properties(
    application_args: &ApplicationArgs,
    application_resources: &ApplicationResources,
) -> ApplicationProperties {
    use serde_yaml::{Value, from_str};

    let config = if let Some(path) = application_args
        .config_location
        .as_ref()
        .filter(|s| !s.is_empty())
        .filter(|s| std::fs::exists(s).is_ok())
    {
        let mut file = std::fs::File::open(&path)
            .map_err(|err| {
                format!(
                    "Failed to open {}: {}\n\
             Action: Check file path and permissions",
                    path, err
                )
            })
            .unwrap();

        let mut buffer = String::new();
        let _ = file.read_to_string(&mut buffer);

        buffer
    } else {
        application_resources
            .load(APPLICATION_CONFIG)
            .map(|data| String::from_utf8(data.to_vec()).unwrap_or_default())
            .unwrap_or_default()
    };

    // check
    if !config.is_empty() {
        match from_str::<ApplicationProperties>(config.as_str()).map(|mut properties| {
            from_str::<Value>(&config)
                .map(|value| properties.set_mapping(value))
                .unwrap_or_default();
            properties
        }) {
            Ok(properties) => return properties,
            Err(_) => (),
        };
    }

    return Default::default();
}

impl From<(&ApplicationArgs, &ApplicationResources)> for ApplicationProperties {
    fn from((args, resources): (&ApplicationArgs, &ApplicationResources)) -> Self {
        into_application_properties(args, resources)
    }
}

impl Default for ApplicationProperties {
    fn default() -> Self {
        use serde_yaml::{Value, from_str, to_string};

        let next = Default::default();

        let mapping = to_string(&next)
            .map(|data| from_str::<Value>(&data).ok())
            .unwrap_or_default();

        Self { next, mapping }
    }
}
