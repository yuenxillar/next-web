use std::sync::Arc;


#[cfg(feature = "rabbitmq_enabled")]
use crate::autoregister::rabbitmq_autoregister::RabbitMQAutoregister;
use next_web_core::autoregister::auto_register::AutoRegister;
#[cfg(feature = "rabbitmq_enabled")]
use next_web_mq::rabbitmq::core::client_properties::RabbitMQClientProperties;

#[cfg(feature = "redis_enabled")]
use super::redis_properties::RedisProperties;
#[cfg(feature = "redis_enabled")]
use crate::autoregister::redis_autoregister::RedisAutoregister;

#[cfg(feature = "minio_enabled")]
use super::minio_properties::MinioProperties;
#[cfg(feature = "minio_enabled")]
use crate::autoregister::minio_autoregister::MinioAutoRegister;
#[cfg(feature = "minio_enabled")]
use super::minio_properties::MinioProperties;

#[cfg(feature = "database_enabled")]
use super::datasource_properties::DataSourceProperties;
#[cfg(feature = "database_enabled")]
use crate::autoregister::database_autoregister::DatabaseAutoRegister;


#[derive(Debug, Clone, serde::Deserialize)]
pub struct DataProperties {
    #[cfg(feature = "database_enabled")]
    datasource: Option<Vec<DataSourceProperties>>,
    #[cfg(feature = "redis_enabled")]
    redis: Option<RedisProperties>,
    #[cfg(feature = "minio_enabled")]
    minio: Option<MinioProperties>,
    #[cfg(feature = "rabbitmq_enabled")]
    rabbitmq: Option<RabbitMQClientProperties>,

}

impl DataProperties {
    #[cfg(feature = "database_enabled")]
    pub fn datasource(&self) -> Option<&Vec<DataSourceProperties>> {
        self.datasource.as_ref()
    }

    #[cfg(feature = "redis_enabled")]
    pub fn redis(&self) -> Option<&RedisProperties> {
        self.redis.as_ref()
    }

    #[cfg(feature = "minio_enabled")]
    pub fn minio(&self) -> Option<&MinioProperties> {
        self.minio.as_ref()
    }

    #[cfg(feature = "rabbitmq_enabled")]
    pub fn rabbitmq(&self) -> Option<&RabbitMQClientProperties> {
        self.rabbitmq.as_ref()
    }

    pub fn registrable(&self) -> Vec<Option<Arc<dyn AutoRegister>>> {
        vec![
            #[cfg(feature = "database_enabled")]
            self.datasource()
                .map(|v| Arc::new(DatabaseAutoRegister(v.clone())) as Arc<dyn AutoRegister>),
            #[cfg(feature = "redis_enabled")]
            self.redis()
                .map(|v| Arc::new(RedisAutoregister(v.clone())) as Arc<dyn AutoRegister>),
            #[cfg(feature = "minio_enabled")]
            self.minio()
                .map(|v| Arc::new(MinioAutoRegister(v.clone())) as Arc<dyn AutoRegister>),
            #[cfg(feature = "rabbitmq_enabled")]
            self.rabbitmq()
                .map(|v| Arc::new(RabbitMQAutoregister(v.clone())) as Arc<dyn AutoRegister>),
        ]
    }
}
