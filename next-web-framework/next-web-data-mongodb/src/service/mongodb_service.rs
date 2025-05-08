use std::{ops::Deref, time::Duration};

use mongodb::{
    Client,
    options::{ClientOptions, Compressor, Credential, ServerAddress},
};
use next_web_core::core::service::Service;

use crate::properties::mongodb_properties::MongodbClientProperties;

#[derive(Clone)]
pub struct MongodbService {
    properties: MongodbClientProperties,
    client: Client,
}

impl Service for MongodbService {
    fn service_name(&self) -> String {
        "mongodbService".into()
    }
}

impl MongodbService {
    pub fn new(properties: MongodbClientProperties) -> Self {
        let client = Self::build_client(&properties);
        Self { properties, client }
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
                if config.username().is_some() && config.password().is_some() {
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
}

impl Deref for MongodbService {
    type Target = Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
