use std::ops::Deref;

use next_web_core::traits::{service::Service, singleton::Singleton};
use rbatis::{Error, RBatis};
use rbdc::pool::Pool;

use crate::autoconfigure::data_source_properties::DataSourceProperties;

#[derive(Clone)]
pub struct DatabaseService {
    properties: DataSourceProperties,
    pub(crate) rbs: RBatis,
}

impl Singleton for DatabaseService {}
impl Service for DatabaseService {}

impl DatabaseService {
    pub fn new(properties: DataSourceProperties) -> Result<Self, Error> {
        let rbs = Self::build_client(&properties)?;
        Ok(Self { properties, rbs })
    }

    fn build_client(config: &DataSourceProperties) -> Result<RBatis, Error> {
        use rbdc::pool::ConnectionManager;
        use rbdc_pool_fast::FastPool;

        // let id = generate_datasource_id(var.id());
        let driver = config.driver_name();

        let rbs = match driver.to_lowercase().as_str() {
            #[cfg(feature = "enable_mysql")]
            "mysql" => {
                use rbdc_mysql::MysqlDriver;
                use rbdc_mysql::options::MySqlConnectOptions;

                let rbs = rbatis::RBatis::new();

                let opts = MySqlConnectOptions::new()
                    .username(config.username().unwrap_or("root"))
                    .password(config.password().unwrap_or_default());
                let mut pool = FastPool::new(ConnectionManager::new_options(MysqlDriver {}, opts))?;
                // Self::configure_pool(&mut pool, config);
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
                    config.driver_name()
                )));
            }
        };

        Ok(rbs)
    }

    // fn configure_pool(pool: &mut rbdc_pool_fast::FastPool, config: &DataSourceProperties) {
    //     if let Some(max_connections) = config.max_connections() {
    //         pool.inner.set_max_idle_conns(max_connections);
    //     }

    //     pool.inner.

    //     pool.inner.set_timeout_check(duration);
    //     if let Some(acquire_timeout) = config.acquire_timeout() {
    //         pool.timeout
    //             .store(Some(Duration::from_secs(acquire_timeout)));
    //     }

    //     pool.timeout
    // }

    pub fn client(&self) -> &RBatis {
        &self.rbs
    }

    pub(crate) fn mut_client(&mut self) -> &mut RBatis {
        &mut self.rbs
    }

    pub fn properties(&self) -> &DataSourceProperties {
        &self.properties
    }
}

impl Deref for DatabaseService {
    type Target = RBatis;
    fn deref(&self) -> &Self::Target {
        &self.rbs
    }
}
