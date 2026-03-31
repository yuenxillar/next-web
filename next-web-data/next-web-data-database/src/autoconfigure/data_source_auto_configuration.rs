use std::sync::Arc;

use next_web_core::{
    context::application_context::ApplicationContext,
    error::BoxError,
    traits::{config::auto_configuration::AutoConfiguration, singleton::Singleton},
};
use rbatis::{Intercept, async_trait};
use rudi_dev::singleton;

use crate::{
    autoconfigure::{
        data_source_properties::DataSourceProperties,
        rbs_connection_properties::RbsConnectionProperties,
    },
    interceptor::block_attack_inner_interceptor::BlockAttackInnerInterceptor,
    service::database_service::DatabaseService,
};

/// Auto-configuration for DataSource.
#[singleton(binds = [Self::into_autoc_configuration])]
#[derive(Clone)]
pub struct DataSourceAutoConfiguration {
    pub data_source_properties: DataSourceProperties,

    #[autowired(option)]
    pub rbs_connection_properties: Option<RbsConnectionProperties>,
}

impl DataSourceAutoConfiguration {
    fn into_autoc_configuration(self: Self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for DataSourceAutoConfiguration {
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), BoxError> {
        // Clone theconfiguration properties
        let data_source_properties = self.data_source_properties.clone();

        let mut database_service = DatabaseService::new(data_source_properties)?;

        // Search for interceptors implemented by users
        let mut intercepts = ctx.resolve_by_type::<Arc<dyn Intercept>>();

        let default_database_interceptor = Arc::new(BlockAttackInnerInterceptor::default())
            as Arc<dyn rbatis::intercept::Intercept>;

        intercepts.insert(0, default_database_interceptor);
        database_service.rbs.set_intercepts(intercepts);

        // Check  status
        database_service
            .exec("SELECT 1", Default::default())
            .await?;

        // Insert the  service into the context and name it with the singleton name
        let name = database_service.singleton_name();
        ctx.insert_singleton_with_name(database_service, name);

        Ok(())
    }
}

#[allow(unused)]
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
