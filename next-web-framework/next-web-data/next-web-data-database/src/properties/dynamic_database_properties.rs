use std::collections::HashMap;

use rudi_dev::{Properties, Singleton};

use crate::properties::database_properties::DatabaseClientProperties;

/// Properties for Dynamic Database client.
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.database.dynamic", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DynamicDatabaseProperties {
    /// This is necessary and do not change the HashMap structure
    base: HashMap<String, DatabaseClientProperties>,

}

impl DynamicDatabaseProperties {
    pub fn base(&self) -> &HashMap<String, DatabaseClientProperties> {
        &self.base
    }
}