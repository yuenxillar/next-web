use std::sync::Arc;

use next_web_core::{
    ApplicationContext, AutoRegister, async_trait, context::properties::ApplicationProperties,
};
use rudi_dev::Singleton;

use crate::{
    properties::minio_properties::MinioClientProperties, service::minio_service::MinioService,
};

/// Register the `DatabaseService` as a singleton with the `DatabaseServiceAutoRegister` type.
#[Singleton(binds = [Self::into_auto_register])]
#[derive(Clone)]
pub struct MinioServiceAutoRegister(pub MinioClientProperties);

impl MinioServiceAutoRegister {
    /// Convert the current structure into a dynamically dispatched `AutoRegister` type
    fn into_auto_register(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[async_trait]
impl AutoRegister for MinioServiceAutoRegister {
    /// Return the singleton name to identify the service
    fn singleton_name(&self) -> &'static str {
        "minioService"
    }

    /// Asynchronous registration method
    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Clone theconfiguration properties
        let client_properties = self.0.clone();

        let minio_service = MinioService::new(client_properties);

        // Create the buckets and check status
        if let Some(buckets) = minio_service.properties().buckets() {
            for name in buckets.iter() {
                if minio_service.bucket_exists(name).await? {
                    continue;
                } else {
                    minio_service.make_bucket(name, false).await?;
                }
            }
        }

        // Insert the  service into the context and name it with the singleton name
        ctx.insert_singleton_with_name(minio_service, self.singleton_name());

        Ok(())
    }
}
