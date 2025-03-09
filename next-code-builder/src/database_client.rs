use crate::database_meta::{DatabaseMeta, DatabaseType};
use rbatis::{dark_std::sync::vec, RBatis};

pub struct DatabaseClient {
    pub rb: RBatis,
    pub database_meta: DatabaseMeta,
}

impl DatabaseClient {
    pub async fn connection(meta: &DatabaseMeta) -> Result<Self, rbatis::Error> {
        let rb = RBatis::new();
        let database_meta = meta.clone();
        match meta.database_type {
            DatabaseType::MYSQL => rb.init(rbdc_mysql::MysqlDriver {}, &meta.url)?,
            DatabaseType::POSTGRES => rb.init(rbdc_pg::PgDriver {}, &meta.url)?,
        }

        Ok(Self { rb, database_meta })
    }

    pub async fn query_version(&self) -> Result<(), rbatis::Error> {
        self.rb.query("SELECT version()", vec![]).await?;
        Ok(())
    }

    pub async fn query_table_info(&self) -> Result<rbs::Value, rbatis::Error> {
        let (sql, param) = match self.database_meta.database_type {
            DatabaseType::POSTGRES => (
                "SELECT 
    c.column_name,
    c.data_type,
    c.character_maximum_length, -- 对于字符类型，显示最大长度
    c.is_nullable,
    c.column_default, -- 列的默认值
    COALESCE(d.description, '') AS column_comment -- 字段的注释
FROM 
    information_schema.columns c
LEFT JOIN 
    pg_catalog.pg_attribute a ON a.attname = c.column_name
LEFT JOIN 
    pg_catalog.pg_class cl ON cl.oid = a.attrelid
LEFT JOIN 
    pg_catalog.pg_namespace n ON n.oid = cl.relnamespace
LEFT JOIN 
    pg_catalog.pg_description d ON d.objoid = cl.oid AND d.objsubid = a.attnum
WHERE 
    c.table_name = ?
    AND cl.relname = ?
    AND n.nspname = 'public'
ORDER BY 
    c.ordinal_position;",
                vec![
                    rbs::to_value!(&self.database_meta.table_name),
                    rbs::to_value!(&self.database_meta.table_name),
                ],
            ),
            DatabaseType::MYSQL => (
                "SELECT column_name, column_type, is_nullable, column_default, column_comment
FROM information_schema.columns
WHERE table_schema = ? AND table_name = ?;",
                vec![
                    rbs::to_value!(&self.database_meta.database_name),
                    rbs::to_value!(&self.database_meta.table_name),
                ],
            ),
        };

        self.rb.query(sql, param).await
    }

    pub async fn order(&self) -> Result<rbs::Value, rbatis::Error> {
        let (sql, param) = match self.database_meta.database_type {
            DatabaseType::POSTGRES => (
                "SELECT column_name, ordinal_position
FROM information_schema.columns
WHERE table_schema = 'public' AND table_name = ?
ORDER BY ordinal_position;",
                vec![rbs::to_value!(&self.database_meta.table_name)],
            ),
            DatabaseType::MYSQL => (
                "SELECT COLUMN_NAME, ORDINAL_POSITION
FROM INFORMATION_SCHEMA.COLUMNS
WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
ORDER BY ORDINAL_POSITION;",
                vec![
                    rbs::to_value!(&self.database_meta.database_name),
                    rbs::to_value!(&self.database_meta.table_name),
                ],
            ),
        };

        self.rb.query(&sql, param).await
    }

    pub async fn query_table_comment(&self) -> Result<rbs::Value, rbatis::Error> {
        let (sql, param) = match self.database_meta.database_type {
            DatabaseType::POSTGRES => (
                format!(
                    "SELECT obj_description('public.{}'::regclass) AS table_comment;",
                    &self.database_meta.table_name
                ),
                vec![rbs::to_value!("")],
            ),
            DatabaseType::MYSQL => (
                "SELECT table_name, table_comment
                FROM information_schema.tables
                WHERE table_schema = ? AND table_name = ?;"
                    .to_string(),
                vec![
                    rbs::to_value!(&self.database_meta.database_name),
                    rbs::to_value!(&self.database_meta.table_name),
                ],
            ),
        };

        self.rb.query(&sql, param).await
    }

    pub fn fields_order(&self, order: &rbs::Value) -> Vec<String> {
        let mut var = String::new();
        let mut comments: Vec<String> = order
            .into_iter()
            .flat_map(|i| i.1.into_iter().map(|x| x.1.to_string()))
            .collect();

        comments.reverse();
        comments.iter().for_each(|f| {
            var.push_str(f.replace("\"", "").as_str());
            var.push_str(" ");
        });

        let mut name_order: Vec<String> = Vec::new();
        let mut temp_number = 1;

        // order
        order.into_iter().for_each(|i| {
            i.1.into_iter().for_each(|f| {
                if temp_number % 2 == 0 {
                    name_order.push(f.1.to_string().replace("\"", ""));
                }
                temp_number += 1;
            })
        });
        name_order
    }
}
