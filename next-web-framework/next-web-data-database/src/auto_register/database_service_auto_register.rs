use std::sync::Arc;

use next_web_core::{
    ApplicationContext, AutoRegister, async_trait, context::properties::ApplicationProperties,
};
use rudi_dev::Singleton;

use crate::{
    core::interceptor::default_database_interceptor::DefaultDatabaseInterceptor,
    properties::database_properties::DatabaseClientProperties,
    service::database_service::DatabaseService,
};

/// Register the `DatabaseService` as a singleton with the `DatabaseServiceAutoRegister` type.
#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct DatabaseServiceAutoRegister(pub DatabaseClientProperties);

impl DatabaseServiceAutoRegister {
    /// Convert the current structure into a dynamically dispatched `AutoRegister` type
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for DatabaseServiceAutoRegister {
    /// Return the singleton name to identify the service
    fn singleton_name(&self) -> &'static str {
        "databaseService"
    }

    /// Asynchronous registration method
    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Clone theconfiguration properties
        let client_properties = self.0.clone();

        let mut database_service = DatabaseService::new(client_properties);

        let var = database_service.get_client_mut();

        // Find interceptors
        let mut intercepts = ctx.resolve_by_type::<Arc<dyn rbatis::intercept::Intercept>>();

        if !intercepts.is_empty() {}
        let default_database_interceptor = Arc::new(DefaultDatabaseInterceptor::default())
            as Arc<dyn rbatis::intercept::Intercept>;

        intercepts.insert(0, default_database_interceptor);
        var.set_intercepts(intercepts);

        // Check  status
        database_service.exec("SELECT 1", vec![]).await?;

        // Insert the  service into the context and name it with the singleton name
        ctx.insert_singleton_with_name(database_service, self.singleton_name());

        Ok(())
    }
}


fn generate_datasource_id(id: &str) -> String {
    let binding = id.to_lowercase();
    let database_id = binding.as_str();
    if database_id.is_empty() {
        return String::from("dataSourceSlave");
    }
    // Capitalize the first letter of id
    let first_str = database_id[0..1].to_uppercase();
    let mut suffix = String::from(first_str);
    suffix.push_str(&database_id[1..]);

    format!("dataSource{}", suffix)
}