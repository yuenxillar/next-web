use crate::{
    autoconfigure::context::mongodb_properties::MongoDBProperties,
    manager::mongodb_manager::MongoDBManager,
};

use futures::executor::block_on;
use mongodb::Client;
use next_web_core::{
    autoregister::auto_register::AutoRegister,
    context::{application_context::ApplicationContext, properties::ApplicationProperties},
};
use next_web_core::async_trait;
use tracing::info;

pub struct MongoDBAutoRegister(pub MongoDBProperties);

#[async_trait]
impl AutoRegister for MongoDBAutoRegister {
    fn singleton_name(&self) -> &'static str {
        "MongoDBAutoRegister"
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "mongodb://{}:{}@{}:{}/{}?authSource=admin&connectTimeoutMS=5000&compressors=zstd",
            self.0.username().unwrap_or_default(),
            self.0.password().unwrap_or_default(),
            self.0.host().unwrap_or("localhost"),
            self.0.port().unwrap_or(27017),
            self.0.database().unwrap_or_default()
        );
        let client = block_on(async move { Client::with_uri_str(url).await.unwrap() });
        let database = self.0.database().unwrap_or_default();

        let mongodb = if database.is_empty() {
            client.default_database().unwrap()
        } else {
            client.database(database)
        };

        // Insert client into application context
        let mongodb_manager = MongoDBManager::new(mongodb, self.0.clone());
        ctx.insert_singleton_with_name::<MongoDBManager, String>(
            mongodb_manager,
            String::from("mongodbManager"),
        );

        info!("Mongodb client registered successfully");
        Ok(())
    }
}
