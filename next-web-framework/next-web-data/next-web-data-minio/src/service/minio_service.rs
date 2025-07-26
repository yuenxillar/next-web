use std::ops::Deref;

use minio_rsc::{provider::StaticProvider, Minio};
use next_web_core::interface::{service::Service, singleton::Singleton};

use crate::properties::minio_properties::MinioClientProperties;

#[derive(Clone)]
pub struct MinioService {
    properties: MinioClientProperties,
    client: Minio,
}

impl Singleton  for MinioService {}
impl Service    for MinioService {}

impl MinioService {
    pub fn new(properties: MinioClientProperties) -> Self {
        let client = Self::build_client(&properties);
        Self { properties, client }
    }

    fn build_client(config: &MinioClientProperties) -> Minio {
        let provider = StaticProvider::new(config.access_key(), config.secret_key(), None);
        let client = Minio::builder()
            .endpoint(config.endpoint())
            .provider(provider)
            .secure(false)
            .build()
            .unwrap();

        client
    }

    pub fn get_client(&self) -> &Minio {
        &self.client
    }

    pub(crate) fn get_client_mut(&mut self) -> &mut Minio {
        &mut self.client
    }

    pub fn properties(&self) -> &MinioClientProperties {
        &self.properties
    }
}


impl Deref for MinioService {
    
    type Target = Minio;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}