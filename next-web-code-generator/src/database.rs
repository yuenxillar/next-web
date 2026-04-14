use std::collections::BTreeMap;

use sqlx::{MySqlPool, Row};

#[cfg(feature = "postgres")]
use sqlx::PgPool;

use crate::config::DataSourceConfig;
use crate::error::Result;

#[cfg(not(feature = "postgres"))]
use crate::error::CodegenError;
use crate::model::{
    ColumnInfo, DatabaseKind, TableInfo, map_rust_type, to_camel_case, to_pascal_case,
    to_snake_case,
};

/// Database metadata introspector for MySQL and PostgreSQL.
#[derive(Debug, Default)]
pub struct DatabaseIntrospector;

impl DatabaseIntrospector {
    /// Read table metadata for the provided datasource configuration.
    pub async fn introspect(datasource: &DataSourceConfig) -> Result<Vec<TableInfo>> {
        let schema = datasource.schema_name()?;

        let (mut tables, mut column_map) = match datasource.kind {
            DatabaseKind::Mysql => {
                let pool = connect_mysql(datasource).await?;
                let tables = load_mysql_tables(&pool, datasource, &schema).await?;
                let columns = load_mysql_columns(&pool, datasource, &schema).await?;
                (tables, columns)
            }
            DatabaseKind::Postgres => {
                #[cfg(feature = "postgres")]
                {
                    let pool = connect_postgres(datasource).await?;
                    let tables = load_postgres_tables(&pool, datasource, &schema).await?;
                    let columns = load_postgres_columns(&pool, datasource, &schema).await?;
                    (tables, columns)
                }
                #[cfg(not(feature = "postgres"))]
                {
                    return Err(CodegenError::UnsupportedDatabase(
                        "postgres feature is not enabled".to_string(),
                    ));
                }
            }
        };

        for table in &mut tables {
            if let Some(columns) = column_map.remove(&table.name) {
                table.primary_keys = columns
                    .iter()
                    .filter(|column| column.primary_key)
                    .map(|column| column.name.clone())
                    .collect();
                table.columns = columns;
            }
        }

        Ok(tables)
    }
}

async fn connect_mysql(datasource: &DataSourceConfig) -> Result<MySqlPool> {
    Ok(MySqlPool::connect(&datasource.connection_url()?).await?)
}

#[cfg(feature = "postgres")]
async fn connect_postgres(datasource: &DataSourceConfig) -> Result<PgPool> {
    Ok(PgPool::connect(&datasource.connection_url()?).await?)
}

async fn load_mysql_tables(
    pool: &MySqlPool,
    datasource: &DataSourceConfig,
    schema: &str,
) -> Result<Vec<TableInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT table_name, table_comment
        FROM information_schema.tables
        WHERE table_schema = ?
          AND table_type = 'BASE TABLE'
        ORDER BY table_name
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    let requested_tables = requested_tables(datasource);
    let mut tables = Vec::new();

    for row in rows {
        let table_name: String = row.try_get("table_name")?;
        if !requested_tables.is_empty() && !requested_tables.contains(&table_name) {
            continue;
        }

        let comment = row
            .try_get::<Option<String>, _>("table_comment")?
            .filter(|text| !text.trim().is_empty());

        tables.push(TableInfo {
            module_name: to_snake_case(&table_name),
            camel_name: to_camel_case(&table_name),
            pascal_name: to_pascal_case(&table_name),
            comment,
            name: table_name,
            columns: Vec::new(),
            primary_keys: Vec::new(),
        });
    }

    Ok(tables)
}

async fn load_mysql_columns(
    pool: &MySqlPool,
    datasource: &DataSourceConfig,
    schema: &str,
) -> Result<BTreeMap<String, Vec<ColumnInfo>>> {
    let rows = sqlx::query(
        r#"
        SELECT table_name,
               column_name,
               ordinal_position,
               data_type,
               column_type,
               is_nullable,
               column_key,
               column_default,
               column_comment,
               extra
        FROM information_schema.columns
        WHERE table_schema = ?
        ORDER BY table_name, ordinal_position
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    let requested_tables = requested_tables(datasource);
    let mut table_columns: BTreeMap<String, Vec<(usize, ColumnInfo)>> = BTreeMap::new();

    for row in rows {
        let table_name: String = row.try_get("table_name")?;
        if !requested_tables.is_empty() && !requested_tables.contains(&table_name) {
            continue;
        }

        let ordinal_position: i64 = row.try_get("ordinal_position")?;
        let column_name: String = row.try_get("column_name")?;
        let data_type: String = row.try_get("data_type")?;
        let column_type: String = row.try_get("column_type")?;
        let is_nullable: String = row.try_get("is_nullable")?;
        let column_key: Option<String> = row.try_get("column_key")?;

        let nullable = is_nullable.eq_ignore_ascii_case("yes");
        let primary_key = column_key
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case("pri"))
            .unwrap_or(false);

        let column = ColumnInfo {
            name: column_name.clone(),
            rust_name: to_snake_case(&column_name),
            pascal_name: to_pascal_case(&column_name),
            data_type: data_type.clone(),
            column_type,
            rust_type: map_rust_type(DatabaseKind::Mysql, &data_type, nullable),
            nullable,
            primary_key,
            default_value: row.try_get("column_default")?,
            comment: row
                .try_get::<Option<String>, _>("column_comment")?
                .filter(|text| !text.trim().is_empty()),
            extra: row
                .try_get::<Option<String>, _>("extra")?
                .filter(|text| !text.trim().is_empty()),
        };

        table_columns
            .entry(table_name)
            .or_default()
            .push((ordinal_position as usize, column));
    }

    Ok(finalize_columns(table_columns))
}

#[cfg(feature = "postgres")]
async fn load_postgres_tables(
    pool: &PgPool,
    datasource: &DataSourceConfig,
    schema: &str,
) -> Result<Vec<TableInfo>> {
    let rows = sqlx::query(
        r#"
        SELECT t.table_name,
               COALESCE(obj_description((quote_ident(t.table_schema) || '.' || quote_ident(t.table_name))::regclass::oid, 'pg_class'), '') AS table_comment
        FROM information_schema.tables t
        WHERE t.table_schema = $1
          AND t.table_type = 'BASE TABLE'
        ORDER BY t.table_name
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    let requested_tables = requested_tables(datasource);
    let mut tables = Vec::new();

    for row in rows {
        let table_name: String = row.try_get("table_name")?;
        if !requested_tables.is_empty() && !requested_tables.contains(&table_name) {
            continue;
        }

        let comment = row
            .try_get::<Option<String>, _>("table_comment")?
            .filter(|text| !text.trim().is_empty());

        tables.push(TableInfo {
            module_name: to_snake_case(&table_name),
            camel_name: to_camel_case(&table_name),
            pascal_name: to_pascal_case(&table_name),
            comment,
            name: table_name,
            columns: Vec::new(),
            primary_keys: Vec::new(),
        });
    }

    Ok(tables)
}

#[cfg(feature = "postgres")]
async fn load_postgres_columns(
    pool: &PgPool,
    datasource: &DataSourceConfig,
    schema: &str,
) -> Result<BTreeMap<String, Vec<ColumnInfo>>> {
    let rows = sqlx::query(
        r#"
        SELECT c.table_name,
               c.column_name,
               c.ordinal_position,
               c.data_type,
               c.udt_name AS column_type,
               c.is_nullable,
               COALESCE(tc.constraint_type, '') AS column_key,
               c.column_default,
               COALESCE(col_description((quote_ident(c.table_schema) || '.' || quote_ident(c.table_name))::regclass::oid, c.ordinal_position), '') AS column_comment,
               '' AS extra
        FROM information_schema.columns c
        LEFT JOIN information_schema.key_column_usage kcu
          ON c.table_schema = kcu.table_schema
         AND c.table_name = kcu.table_name
         AND c.column_name = kcu.column_name
        LEFT JOIN information_schema.table_constraints tc
          ON kcu.constraint_name = tc.constraint_name
         AND kcu.table_schema = tc.table_schema
         AND kcu.table_name = tc.table_name
        WHERE c.table_schema = $1
        ORDER BY c.table_name, c.ordinal_position
        "#,
    )
    .bind(schema)
    .fetch_all(pool)
    .await?;

    let requested_tables = requested_tables(datasource);
    let mut table_columns: BTreeMap<String, Vec<(usize, ColumnInfo)>> = BTreeMap::new();

    for row in rows {
        let table_name: String = row.try_get("table_name")?;
        if !requested_tables.is_empty() && !requested_tables.contains(&table_name) {
            continue;
        }

        let ordinal_position: i32 = row.try_get("ordinal_position")?;
        let column_name: String = row.try_get("column_name")?;
        let data_type: String = row.try_get("data_type")?;
        let column_type: String = row.try_get("column_type")?;
        let is_nullable: String = row.try_get("is_nullable")?;
        let column_key: String = row.try_get("column_key")?;

        let nullable = is_nullable.eq_ignore_ascii_case("yes");
        let primary_key = column_key.eq_ignore_ascii_case("primary key");

        let column = ColumnInfo {
            name: column_name.clone(),
            rust_name: to_snake_case(&column_name),
            pascal_name: to_pascal_case(&column_name),
            data_type: data_type.clone(),
            column_type,
            rust_type: map_rust_type(DatabaseKind::Postgres, &data_type, nullable),
            nullable,
            primary_key,
            default_value: row.try_get("column_default")?,
            comment: row
                .try_get::<Option<String>, _>("column_comment")?
                .filter(|text| !text.trim().is_empty()),
            extra: row
                .try_get::<Option<String>, _>("extra")?
                .filter(|text| !text.trim().is_empty()),
        };

        table_columns
            .entry(table_name)
            .or_default()
            .push((ordinal_position as usize, column));
    }

    Ok(finalize_columns(table_columns))
}

fn finalize_columns(
    table_columns: BTreeMap<String, Vec<(usize, ColumnInfo)>>,
) -> BTreeMap<String, Vec<ColumnInfo>> {
    table_columns
        .into_iter()
        .map(|(table, mut columns)| {
            columns.sort_by_key(|(ordinal, _)| *ordinal);
            (
                table,
                columns
                    .into_iter()
                    .map(|(_, column)| column)
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn requested_tables(datasource: &DataSourceConfig) -> Vec<String> {
    datasource
        .tables
        .iter()
        .map(|table| table.to_string())
        .collect()
}
