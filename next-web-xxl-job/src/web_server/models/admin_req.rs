use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegistryParam {
    pub registry_group: String,
    pub registry_key: String,
    pub registry_value: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CallbackParam {
    pub log_id: u64,
    pub log_date_tim: i64,
    pub handle_code: i32,
    pub handle_msg: Option<String>,
}
