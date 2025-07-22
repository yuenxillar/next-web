use std::sync::Arc;

use next_web_core::{
    ApplicationContext, AutoRegister, async_trait, context::properties::ApplicationProperties,
    core::singleton::Singleton,
};
use rudi_dev::Singleton;

use crate::{
    properties::elasticsearch_properties::ElasticsearchClientProperties,
    service::elasticsearch_service::ElasticsearchService,
};

/// Register the `ElasticsearchService` as a singleton with the `ElasticsearchServiceAutoRegister` type.
#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct ElasticsearchServiceAutoRegister(pub ElasticsearchClientProperties);

impl ElasticsearchServiceAutoRegister {
    /// Convert the current structure into a dynamically dispatched `AutoRegister` type
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for ElasticsearchServiceAutoRegister {
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

        let elasticsearch_service = ElasticsearchService::new(client_properties);

        let singleton_name = elasticsearch_service.singleton_name();
        ctx.insert_singleton_with_name(elasticsearch_service, singleton_name);

        Ok(())
    }
}
