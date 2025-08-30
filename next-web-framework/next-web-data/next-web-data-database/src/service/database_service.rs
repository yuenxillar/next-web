use std::ops::Deref;

use next_web_core::traits::{group::Group, service::Service, singleton::Singleton};
use rbatis::RBatis;

use crate::properties::database_properties::DatabaseClientProperties;

#[derive(Clone)]
pub struct DatabaseService {
    properties: DatabaseClientProperties,
    rbs: RBatis,
}

impl Group      for DatabaseService {}
impl Singleton  for DatabaseService {}
impl Service    for DatabaseService {}

impl DatabaseService {
    pub fn new(properties: DatabaseClientProperties) -> Self {
        let rbs = Self::build_client(&properties);
        Self { properties, rbs }
    }

    fn build_client(config: &DatabaseClientProperties) -> RBatis {
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
                rbs.init_option::<MysqlDriver, MySqlConnectOptions, FastPool>(MysqlDriver {}, opts)
                    .unwrap();

                rbs
            }
            #[cfg(feature = "enable_postgres")]
            "postgres" => {
                use rbdc_pg::PgDriver;
                use rbdc_pg::options::PgConnectOptions;

                let rbs = rbatis::RBatis::new();
                let url_extra = config.url_extra().unwrap_or_default();
                let var1 = url_extra.split("&").collect::<Vec<&str>>();
                let options = var1
                    .iter()
                    .map(|s| s.split("=").collect::<Vec<&str>>())
                    .map(|n| (n[0], n[1]));

                let opts = PgConnectOptions::new()
                    .port(config.port().unwrap_or(5432))
                    .host(config.host().unwrap_or("localhost"))
                    .username(config.username().unwrap_or("postgres"))
                    .password(config.password().unwrap_or_default())
                    .database(config.database())
                    .options(options);

                rbs.init_option::<PgDriver, PgConnectOptions, FastPool>(PgDriver {}, opts)
                    .unwrap();
                rbs
            }
            _ => {
                panic!("Datasource driver not supported")
            }
        };

        rbs
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