use std::collections::HashMap;

use next_web_macros::properties;
use rudi_dev::singleton;

use crate::properties::database_properties::DatabaseClientProperties;

/// Properties for Dynamic Database client.
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.data.database.dynamic", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DynamicDatabaseProperties {
    /// This is necessary and do not change the HashMap structure
    dynamic: HashMap<String, DatabaseClientProperties>,
}

impl DynamicDatabaseProperties {}
