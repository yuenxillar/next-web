use std::sync::Arc;

use next_web_core::{
    async_trait, context::properties::ApplicationProperties, interface::singleton::Singleton, ApplicationContext, AutoRegister
};
use rudi_dev::Singleton;
use tracing::debug;

use crate::{
    properties::mongodb_properties::MongodbClientProperties,
    service::mongodb_service::MongodbService,
};

/// Register the `DatabaseService` as a singleton with the `DatabaseServiceAutoRegister` type.
#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct MinioServiceAutoRegister(pub MongodbClientProperties);

impl MinioServiceAutoRegister {
    /// Convert the current structure into a dynamically dispatched `AutoRegister` type
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for MinioServiceAutoRegister {
    /// Return the singleton name to identify the service
    fn registered_name(&self) -> &'static str {
        ""
    }

    /// Asynchronous registration method
    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Clone theconfiguration properties
        let client_properties = self.0.clone();

        let mongodb_service = MongodbService::new(client_properties);

        // Print all database names and Check status
        for db_name in mongodb_service.list_database_names().await? {
            debug!("Found mongodb database: {}", db_name);
        }

        // Insert the  service into the context and name it with the singleton name
        let singleton_name = mongodb_service.singleton_name();
        ctx.insert_singleton_with_name(mongodb_service, singleton_name);

        Ok(())
    }
}
