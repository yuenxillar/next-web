use crate::autoregister::auto_register::AutoRegister;
use crate::interceptor::default_database_interceptor::DefaultDatabaseInterceptor;
use crate::middleware::check_status::MiddlewareCheckStatus;
use crate::{
    autoconfigure::context::datasource_properties::DataSourceProperties,
    manager::database_manager::DatabaseManager,
};
use core::panic;
use std::sync::Arc;

pub struct DatabaseAutoRegister(pub Vec<DataSourceProperties>);

impl AutoRegister for DatabaseAutoRegister {

    fn name(&self) -> &'static str {
        "DatabaseAutoRegister"
    }


    fn register(&self, ctx: &mut rudi::Context) -> Result<(), Box<dyn std::error::Error>> {
        use rbdc_pool_fast::FastPool;

        for var in self.0.iter() {
            let url_extra = var.url_extra().unwrap_or_default();
            let id = generate_datasource_id(var.id());
            let mut rbs = match var.driver().to_lowercase().as_str() {
                "mysql" => {
                    use rbdc_mysql::options::MySqlConnectOptions;
                    use rbdc_mysql::MysqlDriver;

                    let rbs = rbatis::RBatis::new();

                    let opts = MySqlConnectOptions::new()
                        .port(var.port())
                        .host(var.host())
                        .username(var.username())
                        .password(var.password())
                        .database(var.database());
                    rbs.init_option::<MysqlDriver, MySqlConnectOptions, FastPool>(
                        MysqlDriver {},
                        opts,
                    )?;

                    rbs
                }
                "postgres" => {
                    use rbdc_pg::options::PgConnectOptions;
                    use rbdc_pg::PgDriver;

                    let rbs = rbatis::RBatis::new();
                    let var1 = url_extra.split("&").collect::<Vec<&str>>();
                    let options = var1
                        .iter()
                        .map(|s| s.split("=").collect::<Vec<&str>>())
                        .map(|n| (n[0], n[1]));
                    let opts = PgConnectOptions::new()
                        .port(var.port())
                        .host(var.host())
                        .username(var.username())
                        .password(var.password())
                        .database(var.database())
                        .options(options);
                    
                    rbs.init_option::<PgDriver, PgConnectOptions, FastPool>(PgDriver {}, opts)?;
                    rbs
                }
                _ => {
                    panic!("datasource driver not supported")
                }
            };

            // default interceptors
            let default_database_interceptor = Arc::new(DefaultDatabaseInterceptor::default())
                as Arc<dyn rbatis::intercept::Intercept>;

            let mut intercept = ctx.resolve_by_type::<Arc<dyn rbatis::intercept::Intercept>>();
            if !intercept.is_empty() {
                intercept.push(default_database_interceptor);
                rbs.set_intercepts(intercept);
            } else {
                rbs.set_intercepts(vec![default_database_interceptor]);
            }

            let rbs = DatabaseManager::new(rbs, var.clone());

            // check  status
            futures::executor::block_on(rbs.status())?;
            
            ctx.insert_singleton_with_name::<DatabaseManager, String>(rbs, id.clone());
            println!(
                "DatabaseAutoRegister registered successfully! datasource id: {}",
                id
            );
        }
        Ok(())
    }
}
impl DatabaseAutoRegister {
    pub fn new(datasource_properties: Vec<DataSourceProperties>) -> Self {
        Self {
            0: datasource_properties,
        }
    }
}

fn generate_datasource_id(id: &str) -> String {
    let binding = id.to_lowercase();
    let database_id = binding.as_str();
    if database_id.is_empty() {
        return String::from("dataSourceSlave");
    }
    // Capitalize the first letter of id
    let first_str = database_id[0..1].to_uppercase();
    let mut suffix = String::from(first_str);
    suffix.push_str(&database_id[1..]);

    format!("dataSource{}", suffix)
}
