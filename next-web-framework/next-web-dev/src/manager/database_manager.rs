use crate::autoconfigure::context::datasource_properties::DataSourceProperties;

#[derive(Clone)]
pub struct DatabaseManager {
    rbs: rbatis::RBatis,
    options: DataSourceProperties,
}

impl DatabaseManager {
    pub fn new(rbs: rbatis::RBatis, options: DataSourceProperties) -> Self {
        Self { rbs, options }
    }

    pub fn get_conn(&self) -> &rbatis::RBatis {
        &self.rbs
    }

    pub fn options(&self) -> &DataSourceProperties {
        &self.options
    }
}

use crate::middleware::check_status::MiddlewareCheckStatus;
use async_trait::async_trait;

#[async_trait]
impl MiddlewareCheckStatus for DatabaseManager {
    async fn status(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.rbs.try_acquire_timeout(std::time::Duration::from_secs(2)).await?;
        conn.query("SELECT 1;", vec![]).await?;
        Ok(())
    }
}
