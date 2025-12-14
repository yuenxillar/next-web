use std::{ops::Deref, time::Duration};

use mongodb::{
    Client,
    options::{ClientOptions, Compressor, Credential, ServerAddress},
};
use next_web_core::traits::{service::Service, singleton::Singleton};

use crate::properties::mongodb_properties::MongodbClientProperties;

#[derive(Clone)]
pub struct MongodbService {
    properties: MongodbClientProperties,
    client: Client,
    database: mongodb::Database,
}


impl Singleton  for MongodbService {}
impl Service    for MongodbService {}

impl MongodbService {
    pub fn new(properties: MongodbClientProperties) -> Self {
        let client = Self::build_client(&properties);
        let database = client.database(properties.database().unwrap());
        Self { properties, client, database}
    }

    fn build_client(config: &MongodbClientProperties) -> Client {
        let client_options = ClientOptions::builder()
            .hosts(vec![ServerAddress::Tcp {
                host: config.host().unwrap_or("localhost".to_string()),
                port: config.port(),
            }])
            .compressors(if config.zstd() {
                Some(vec![Compressor::Zstd { level: None }])
            } else {
                None
            })
            .credential(
                if config.username().is_none() && config.password().is_none() {
                    None
                } else {
                    Some(
                        Credential::builder()
                            .username(config.username().map(|s| s.to_string()).unwrap_or_default())
                            .password(config.password().map(|s| s.to_string()).unwrap_or_default())
                            .build(),
                    )
                },
            )
            .connect_timeout(Some(Duration::from_millis(
                config.connect_timeout().unwrap_or(5000),
            )))
            .default_database(config.database().map(|s| s.to_string()).unwrap_or_default())
            .app_name(Some("nextWebMongodbApp".into()))
            .max_pool_size(Some(21))
            .build();
        let client = Client::with_options(client_options).unwrap();

        client
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn get_client_mut(&mut self) -> &mut Client {
        &mut self.client
    }

    pub fn properties(&self) -> &MongodbClientProperties {
        &self.properties
    }

    pub(crate) fn get_database(&self) -> &mongodb::Database {
        &self.database
    }
}

impl Deref for MongodbService {
    type Target = Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
