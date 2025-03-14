use std::path::PathBuf;

use hashbrown::HashMap;

use crate::{
    autoconfigure::context::next_properties::NextProperties,
    util::{command_util::CommandUtil, file_util::FileUtil},
};

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
                return serde_yaml::from_value::<T>(value.clone())
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
}

impl ApplicationProperties {
    pub fn new() -> Self {
        parse_to_application()
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

pub const SERVER_PROFILE_NAME: &str = "server.profile";

pub const RESOURCES_NAME: &str = "resources";

fn parse_to_application() -> ApplicationProperties {
    let parameters = CommandUtil::handle_args(std::env::args().collect());
    let file_path;
    if let Some(var) = parameters.get(SERVER_PROFILE_NAME) {
        file_path = var.to_string();
    } else {
        file_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join(RESOURCES_NAME)
        .join("application.yaml")
        .display().to_string();
    }
    let (mut application, mapping) =
        FileUtil::read_file_into_application::<ApplicationProperties>(&file_path);
    application.set_mapping(mapping);

    application
}

impl Drop for ApplicationProperties {
    fn drop(&mut self) {}
}
