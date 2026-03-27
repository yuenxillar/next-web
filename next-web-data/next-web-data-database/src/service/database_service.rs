use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use next_web_core::traits::{service::Service, singleton::Singleton};
use rbdc::pool::Pool;
use rbatis::{Error, RBatis};

use crate::properties::database_properties::DatabaseClientProperties;

#[derive(Clone)]
pub struct DatabaseService {
    properties: DatabaseClientProperties,
    rbs: RBatis,
}

impl Singleton  for DatabaseService {}
impl Service    for DatabaseService {}

impl DatabaseService {
    pub fn new(properties: DatabaseClientProperties) -> Result<Self, Error> {
        let rbs = Self::build_client(&properties)?;
        Ok(Self { properties, rbs })
    }

    fn build_client(config: &DatabaseClientProperties) -> Result<RBatis, Error> {
        use rbdc::pool::ConnectionManager;
        use rbdc_pool_fast::FastPool;

        // let id = generate_datasource_id(var.id());
        let driver = config.driver();

        let rbs = match driver.to_lowercase().as_str() {
            #[cfg(feature = "enable_mysql")]
            "mysql" => {
                use rbdc_mysql::MysqlDriver;
                use rbdc_mysql::options::MySqlConnectOptions;

                let rbs = rbatis::RBatis::new();

                let opts = MySqlConnectOptions::new()
                    .port(config.port().unwrap_or(3306))
                    .host(config.host().unwrap_or("localhost"))
                    .username(config.username().unwrap_or("root"))
                    .password(config.password().unwrap_or_default())
                    .database(config.database());
                let mut pool = FastPool::new(ConnectionManager::new_arc(
                    Arc::new(Box::new(MysqlDriver {})),
                    Arc::new(Box::new(opts)),
                ))?;
                Self::configure_pool(&mut pool, config);
                rbs.init_pool(pool)?;

                rbs
            }
            #[cfg(feature = "enable_postgres")]
            "postgres" => {
                use rbdc_pg::PgDriver;
                use rbdc_pg::options::PgConnectOptions;

                let rbs = rbatis::RBatis::new();
                let url_extra = config.url_extra().unwrap_or_default();
                let options = url_extra
                    .split('&')
                    .filter_map(|segment| segment.split_once('='));

                let opts = PgConnectOptions::new()
                    .port(config.port().unwrap_or(5432))
                    .host(config.host().unwrap_or("localhost"))
                    .username(config.username().unwrap_or("postgres"))
                    .password(config.password().unwrap_or_default())
                    .database(config.database())
                    .options(options);

                let mut pool = FastPool::new(ConnectionManager::new_arc(
                    Arc::new(Box::new(PgDriver {})),
                    Arc::new(Box::new(opts)),
                ))?;
                Self::configure_pool(&mut pool, config);
                rbs.init_pool(pool)?;
                rbs
            }
            _ => {
                return Err(Error::from(format!(
                    "Datasource driver '{}' is not supported",
                    config.driver()
                )))
            }
        };

        Ok(rbs)
    }

    fn configure_pool(pool: &mut rbdc_pool_fast::FastPool, config: &DatabaseClientProperties) {
        if let Some(max_connections) = config.max_connections() {
            pool.inner.set_max_open(max_connections);
        }

        if let Some(acquire_timeout) = config.acquire_timeout() {
            pool.timeout
                .store(Some(Duration::from_secs(acquire_timeout)));
        }
    }

    pub fn get_client(&self) -> &RBatis {
        &self.rbs
    }

    pub(crate) fn get_client_mut(&mut self) -> &mut RBatis {
        &mut self.rbs
    }

    pub fn properties(&self) -> &DatabaseClientProperties {
        &self.properties
    }
}


impl Deref for DatabaseService {
    type Target = RBatis;
    fn deref(&self) -> &Self::Target {
        &self.rbs
    }
}
