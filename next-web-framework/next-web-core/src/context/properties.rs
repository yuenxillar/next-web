use dyn_clone::DynClone;

use regex::Regex;
use serde::de::value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Read;

use crate::constants::application_constants::{APPLICATION_CONFIG, SECURE_PROPERTIES_MARK};
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
///
pub trait Properties: DynClone + AutoRegister {}

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
    /// use next_web_dev::application::application_properties::ApplicationProperties;
    /// use hashbrown::HashMap;
    ///
    /// let mut props = ApplicationProperties::default();
    /// props.set_mapping(HashMap::from([("key1".to_string(), serde_yaml::Value::String("value1".to_string()))]));
    /// assert_eq!(props.one_value::<String>("key1"), Some("value1".to_string()));
    ///
    pub fn one_value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
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

    pub fn dynamic_value<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Option<HashMap<String, T>> {
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
                                        )
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

    /// Decrypt the properties.
    pub fn decrypt(&mut self) {
        // 不解密关于 Server 相关的配置

        if let Some(mapping) = self.mapping_mut() {
            // println!("mapping: {:#?}", mapping);

            match mapping.as_mapping_mut() {
                Some(mapping) => {
                    let var = mapping.iter_mut().filter(|(_key, value)| match value {
                        serde_yaml::Value::String(s) => {
                            if s.starts_with(SECURE_PROPERTIES_MARK) && &s[3..4] == ":" {}

                            true
                        }
                        serde_yaml::Value::Mapping(mapping) => todo!(),
                        _ => false,
                    });
                }
                None => {}
            }
        }
    }

    /// Replace the placeholders in the properties.
    pub fn replace_placeholders(&mut self) {
        // Two situations
        // ${author.name}  $${MY_ENV_VAR}

        let temporary = self.mapping.clone();
        self.mapping.as_mut().map(|mapping| {
            let mapping = match mapping.as_mapping_mut() {
                Some(mapping) => mapping,
                None => return,
            };

            mapping
                .iter_mut()
                .map(|val| val.1)
                .for_each(|value| helper_function(temporary.as_ref(), value));
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

fn helper_function(temporary: Option<&serde_yaml::Value>, value: &mut serde_yaml::Value) {
    match value {
        serde_yaml::Value::String(s) => {
            let s = s.trim();
            if s.starts_with("${") && s.ends_with("}") {
                let mut temporary = match temporary {
                    Some(temporary) => temporary,
                    None => return,
                };

                let key = s[2..s.len() - 1].to_string();
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

                return;
            }

            if s.starts_with("$${") && s.ends_with("}") {
                let val = s[3..s.len() - 1].to_string();
                let key = val.trim();
                // My suggestion is to panic directly
                let var = match std::env::var(key) {
                        Ok(var) => var,
                        Err(_) => panic!("In the configuration file, the environment variable [{}] cannot be obtained. Please check the environment configuration.", key),
                    };
                *value = serde_yaml::Value::String(var);
            }
        }
        serde_yaml::Value::Mapping(mapping) => {
            mapping
                .iter_mut()
                .for_each(|(_, value)| helper_function(temporary, value));
        }
        _ => return,
    };
}

impl Default for ApplicationProperties {
    fn default() -> Self {
        Self {
            next: NextProperties::default(),
            mapping: None,
        }
    }
}

fn into_application_properties(
    application_args: &ApplicationArgs,
    application_resources: &ApplicationResources,
) -> ApplicationProperties {
    use serde_yaml::Value;

    let config_path: Option<String> = application_args.config_location.clone();

    let mut config = String::new();

    if config_path.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
        let path = config_path.unwrap();
        if let Ok(_) = std::fs::exists(&path) {
            let mut file = std::fs::File::open(&path).unwrap();
            let mut _buffer = String::new();
            file.read_to_string(&mut _buffer).unwrap();

            if _buffer.is_empty() {
                panic!(
                    "The application configuration file is empty, file path: {}",
                    &path
                );
            }

            config = _buffer;
        } else {
            panic!(
                "Please check if the configuration file of the application exists: {:?}",
                &path
            );
        }
    } else {
        if let Some(data) = application_resources.load(APPLICATION_CONFIG) {
            config = String::from_utf8(data.to_vec()).unwrap();
        }
    }

    // replace var
    let mut pre_replace = Vec::new();
    if let Ok(re) = Regex::new(r"\$\{(.*?)\}") {
        re.captures_iter(&config.as_ref()).for_each(|item| {
            item.get(1)
                .map(|s| s.as_str())
                .map(|s1| pre_replace.push(s1.to_string()));
        });
    };

    // TODO

    // mapping value
    let mapping = serde_yaml::from_str::<Value>(&config).unwrap();

    // into application properties
    let mut application_properties: ApplicationProperties =
        serde_yaml::from_str(config.as_str()).unwrap_or_default();
    application_properties.set_mapping(mapping);

    // return
    return application_properties;
}

impl From<(&ApplicationArgs, &ApplicationResources)> for ApplicationProperties {
    fn from((args, resources): (&ApplicationArgs, &ApplicationResources)) -> Self {
        into_application_properties(args, resources)
    }
}

impl Drop for ApplicationProperties {
    fn drop(&mut self) {}
}

dyn_clone::clone_trait_object!(Properties);
