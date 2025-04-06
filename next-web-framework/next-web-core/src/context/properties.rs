use dyn_clone::DynClone;
use hashbrown::HashMap;
use serde::de::DeserializeOwned;

use std::fmt::Debug;
use std::io::Read;
use std::{fs::File, path::PathBuf};

use crate::AutoRegister;

use super::next_properties::NextProperties;

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
    let mut data: HashMap<String, String> = HashMap::new();
    // 解析命令行参数
    // 格式：--key=value 或者 -key2=value2
    let args: Vec<String> = std::env::args().collect();
    for item in args {
        if !item.starts_with("--") || !item.starts_with("-") {
            continue;
        }
        if !item.contains("=") {
            continue;
        }

        let mut split_arg = item.split('=');
        let key = split_arg.next().unwrap().to_string();
        let value = split_arg.next().unwrap().to_string();
        data.insert(key, value);
    }

    let file_path;
    if let Some(var) = data.get(SERVER_PROFILE_NAME) {
        file_path = var.to_string();
    } else {
        file_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join(RESOURCES_NAME)
            .join("application.yaml")
            .display()
            .to_string();
    }
    let (mut application, mapping) =
        read_file_into_application::<ApplicationProperties>(&file_path);
    application.set_mapping(mapping);

    application
}

pub fn read_file_into_application<T: DeserializeOwned + Debug>(
    file_path: &str,
) -> (T, HashMap<String, serde_yaml::Value>) {
    use serde_yaml::Value;

    println!("read application file: {}", file_path);
    match std::fs::metadata(file_path) {
        Ok(_) => (),
        Err(_error) => panic!("The application config file is not exits!!"),
    }
    let mut file = File::open(file_path).expect("application file open is _error!!");
    let mut str = String::new();
    let _ = file.read_to_string(&mut str).unwrap();

    // // replace var
    // let replace_var = |content: &str| -> String {
    //     let re = Regex::new(r"\$\{server_ip\}").unwrap();

    //     // 替换 ${server_ip} 为新的 IP 地址
    //     let updated_content = re.replace_all(&content, "192.168.1.130");
    //     updated_content.to_string()
    // };

    // let buf = replace_var(str.as_str());
    let buf = str;

    // mapping value
    let docs = serde_yaml::from_str::<Value>(&buf).unwrap();
    let mut data_map: HashMap<String, Value> = HashMap::new();

    // Prepare a recursive function to fill in
    fn populate_map(prefix: String, value: &Value, map: &mut HashMap<String, Value>) {
        match value {
            Value::Mapping(mapping) => {
                for (k, v) in mapping {
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

    // Fill in the map
    populate_map(String::new(), &docs, &mut data_map);

    // into application
    let application: T = serde_yaml::from_str(buf.as_str()).unwrap();

    // return
    return (application, data_map);
}

impl Drop for ApplicationProperties {
    fn drop(&mut self) {}
}

dyn_clone::clone_trait_object!(Properties);
