#[derive(Clone)]
pub struct DatabaseMeta {
    pub url: String,
    pub table_name: String,
    pub database_name: String,
    pub database_type: DatabaseType,
}

#[derive(PartialEq, Eq, Clone)]
pub enum DatabaseType {
    MYSQL,
    POSTGRES,
}

impl Default for DatabaseMeta {
    fn default() -> Self {
        Self {
            url: Default::default(),
            table_name: Default::default(),
            database_type: DatabaseType::POSTGRES,
            database_name: Default::default(),
        }
    }
}

impl Default for DatabaseType {
    fn default() -> Self {
        DatabaseType::POSTGRES
    }
}
