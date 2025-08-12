use clap::Parser;
use dyn_clone::DynClone;
use hashbrown::HashMap;

use regex::Regex;
use std::fmt::Debug;
use std::io::Read;

use crate::context::application_args::ApplicationArgs;
// use crate::context::application_resources::ApplicationResources;
// use crate::constants::application_constants::APPLICATION_CONFIG_FILE;


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

impl ApplicationProperties {
    pub fn new() -> Self {
        into_application_properties()
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

pub fn into_application_properties() -> ApplicationProperties {
    use serde_yaml::Value;

    let args = ApplicationArgs::parse();

    let config_path: Option<String> = args.config_location;

    let mut config_data = String::new();

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

            config_data = _buffer;
        } else {
            panic!(
                "Please check if the configuration file of the application exists: {:?}",
                &path
            );
        }
    }
    //  else {
    //     if let Some(var) = ApplicationResources::get(APPLICATION_CONFIG_FILE) {
    //         config_data = String::from_utf8(vec![]).unwrap();
    //     }
    // }

    // replace var
    let mut pre_replace = Vec::new();
    if let Ok(re) = Regex::new(r"\$\{(.*?)\}") {
        re.captures_iter(&config_data.as_ref()).for_each(|item| {
            item.get(1)
                .map(|s| s.as_str())
                .map(|s1| pre_replace.push(s1.to_string()));
        });
    };

    // TODO

    // mapping value
    let values = serde_yaml::from_str::<Value>(&config_data).unwrap();
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
        serde_yaml::from_str(config_data.as_str()).unwrap_or_default();
    application_properties.set_mapping(mapping);

    // return
    return application_properties;
}

impl Drop for ApplicationProperties {
    fn drop(&mut self) {}
}

dyn_clone::clone_trait_object!(Properties);
