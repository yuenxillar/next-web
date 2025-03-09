use mongodb::Database;

use crate::autoconfigure::context::mongodb_properties::MongoDBProperties;


#[derive(Clone)]
pub struct MongoDBManager {
    database: Database,
    properties: MongoDBProperties,
}

impl MongoDBManager  {
    
    pub fn new(database: Database, properties: MongoDBProperties) -> Self {
        Self {
            database,
            properties,
        }
    }

    
    pub fn database(&self) -> &Database {
        &self.database
    }

    pub fn properties(&self) -> &MongoDBProperties {
        &self.properties
    }
}