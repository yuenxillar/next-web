use dyn_clone::DynClone;
use hashbrown::HashMap;

use regex::Regex;
use std::collections::HashSet;
use std::fmt::Debug;
use std::io::Read;

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
///
pub trait Properties: DynClone + AutoRegister {}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApplicationProperties {
    /// This Properties is Mapping data from the configuration file
    next: NextProperties,

    /// Only for register that have not been deserialized
    #[serde(skip_deserializing)]
    mapping_value: Option<HashMap<String, serde_yaml::Value>>,
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
        if let Some(map) = self.mapping_value.as_ref() {
            if let Some(value) = map.get(key) {
                return serde_yaml::from_value::<T>(value.to_owned())
                    .map(|v| Some(v))
                    .unwrap_or_default();
            }
            return None;
        }
        None
    }

    pub fn dynamic_value<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Option<HashMap<String, T>> {
        if let Some(map) = self.mapping_value.as_ref() {
            // 查找key的动态值
            let index = key.split(".").collect::<Vec<_>>().len();
            let keys = map
                .keys()
                .filter(|s| s.starts_with(key))
                .filter(|s| index == (s.split(".").collect::<Vec<_>>().len() + 1))
                .filter_map(|key| {
                    let var = map.get(key).map(|value| {
                        serde_yaml::from_value::<T>(value.to_owned())
                    }.ok()).unwrap_or_default();

                    Some((key.split(".").last().unwrap_or_default(), var.unwrap()))
                })
                .collect::<HashMap<_, _>>();

            // if let Some(value) = map.get(key) {
            //     return serde_yaml::from_value::<T>(value.to_owned())
            //         .map(|v| Some(v))
            //         .unwrap_or_default();
            // }
            return None;
        }
        None
    }

    pub fn set_mapping(&mut self, mapping: HashMap<String, serde_yaml::Value>) {
        if mapping.is_empty() {
            return;
        }
        self.mapping_value = Some(mapping);
    }

    pub fn mapping_value(&self) -> Option<&HashMap<String, serde_yaml::Value>> {
        self.mapping_value.as_ref()
    }
}

impl Default for ApplicationProperties {
    fn default() -> Self {
        Self {
            next: NextProperties::default(),
            mapping_value: None,
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
    let values = serde_yaml::from_str::<Value>(&config).unwrap();
    let mut mapping: HashMap<String, Value> = HashMap::new();

    // prepare a recursive function to fill in
    fn populate_map(prefix: String, value: &Value, map: &mut HashMap<String, Value>) {
        match value {
            Value::Mapping(map_value) => {
                for (k, v) in map_value {
                    if let Some(key) = k.as_str() {
                        populate_map(
                            format!(
                                "{}{}",
                                if prefix.is_empty() {
                                    String::new()
                                } else {
                                    format!("{}.", prefix)
                                },
                                key
                            ),
                            v,
                            map,
                        );
                    }
                }
            }
            _ => {
                map.insert(prefix, value.clone());
            }
        }
    }

    // fill in the map
    populate_map(String::new(), &values, &mut mapping);

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
