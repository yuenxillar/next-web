use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::config::{DataSourceConfig, ProjectConfig};
use crate::error::{CodegenError, Result};
use crate::model::TableInfo;

const RESERVED_TEMPLATE_KEYS: &[&str] =
    &["project", "datasource", "tables", "table", "now", "time"];

/// Strongly typed template context used by the generator and template engine.
///
/// The fixed fields describe the built-in variables that templates can access,
/// while `extra` allows library users to inject their own top-level parameters
/// without changing the core structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    /// Target project metadata.
    pub project: TemplateProjectContext,
    /// Selected datasource metadata.
    pub datasource: DataSourceConfig,
    /// Introspected table collection.
    pub tables: Vec<TableInfo>,
    /// Current table when rendering a per-table template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<TableInfo>,
    /// Backward-compatible alias of the current RFC3339 timestamp.
    pub now: String,
    /// Common time values ready for direct template usage.
    pub time: TemplateTimeContext,
    /// Additional user-defined top-level template parameters.
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

impl TemplateContext {
    /// Build a template context from project config, datasource config and discovered tables.
    pub fn new(
        project: &ProjectConfig,
        datasource: &DataSourceConfig,
        tables: &[TableInfo],
    ) -> Self {
        let time = TemplateTimeContext::now();
        Self {
            project: TemplateProjectContext::from(project),
            datasource: datasource.clone(),
            tables: tables.to_vec(),
            table: None,
            now: time.rfc3339.clone(),
            time,
            extra: Map::new(),
        }
    }

    /// Clone the current context and bind the provided table as `table`.
    pub fn with_table(&self, table: &TableInfo) -> Self {
        let mut context = self.clone();
        context.table = Some(table.clone());
        context
    }

    /// Insert an additional top-level template parameter.
    ///
    /// Reserved keys such as `project` and `table` cannot be overwritten.
    pub fn insert<T>(&mut self, key: impl Into<String>, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(value)?;
        self.insert_value(key, value)
    }

    /// Insert an additional top-level template parameter from a raw JSON value.
    pub fn insert_value(&mut self, key: impl Into<String>, value: Value) -> Result<()> {
        let key = key.into();
        validate_extra_key(&key)?;
        self.extra.insert(key, value);
        Ok(())
    }

    /// Serialize the context into the JSON value consumed by the template engine.
    pub fn to_value(&self) -> Result<Value> {
        Ok(serde_json::to_value(self)?)
    }

    /// Return the reserved top-level template parameter names.
    pub fn reserved_keys() -> &'static [&'static str] {
        RESERVED_TEMPLATE_KEYS
    }
}

/// Stable project data exposed to templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateProjectContext {
    /// Logical project name from the generator config.
    pub name: String,
    /// Output root directory converted into a string for template interpolation.
    pub output_dir: String,
}

impl From<&ProjectConfig> for TemplateProjectContext {
    fn from(project: &ProjectConfig) -> Self {
        Self {
            name: project.name.clone(),
            output_dir: project.output_dir.to_string_lossy().to_string(),
        }
    }
}

/// Common time values exposed to templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateTimeContext {
    /// Current UTC timestamp in RFC3339 format.
    pub rfc3339: String,
    /// Current UTC timestamp in `yyyy-MM-dd` format.
    pub date: String,
    /// Current UTC time in `HH:mm:ss` format.
    pub clock: String,
    /// Current UTC timestamp in `yyyy-MM-dd HH:mm:ss` format.
    pub datetime: String,
    /// Unix timestamp in seconds.
    pub timestamp: i64,
    /// Unix timestamp in milliseconds.
    pub timestamp_millis: i64,
    /// Current UTC year.
    pub year: i32,
    /// Current UTC month.
    pub month: u32,
    /// Current UTC day.
    pub day: u32,
    /// Current UTC hour.
    pub hour: u32,
    /// Current UTC minute.
    pub minute: u32,
    /// Current UTC second.
    pub second: u32,
}

impl TemplateTimeContext {
    /// Build the time context using the current UTC time.
    pub fn now() -> Self {
        Self::from_datetime(Utc::now())
    }

    /// Build the time context from a concrete UTC datetime.
    pub fn from_datetime(datetime: DateTime<Utc>) -> Self {
        Self {
            rfc3339: datetime.to_rfc3339(),
            date: datetime.format("%Y-%m-%d").to_string(),
            clock: datetime.format("%H:%M:%S").to_string(),
            datetime: datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
            timestamp: datetime.timestamp(),
            timestamp_millis: datetime.timestamp_millis(),
            year: datetime.year(),
            month: datetime.month(),
            day: datetime.day(),
            hour: datetime.hour(),
            minute: datetime.minute(),
            second: datetime.second(),
        }
    }
}

fn validate_extra_key(key: &str) -> Result<()> {
    if key.trim().is_empty() {
        return Err(CodegenError::InvalidTemplateKey(
            "template parameter key cannot be empty".to_string(),
        ));
    }

    if RESERVED_TEMPLATE_KEYS
        .iter()
        .any(|reserved| reserved == &key)
    {
        return Err(CodegenError::ReservedTemplateKey(key.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::TemplateContext;
    use crate::config::{DataSourceConfig, ProjectConfig};
    use crate::model::{DatabaseKind, TableInfo};

    #[test]
    fn template_context_supports_extra_fields() {
        let mut context = TemplateContext::new(
            &ProjectConfig {
                name: "demo".to_string(),
                output_dir: "./generated".into(),
                overwrite: false,
                modules: Default::default(),
            },
            &DataSourceConfig {
                name: "db1".to_string(),
                kind: DatabaseKind::Mysql,
                url: None,
                host: Some("127.0.0.1".to_string()),
                port: Some(3306),
                database: Some("demo".to_string()),
                schema: None,
                username: Some("root".to_string()),
                password: Some("123456".to_string()),
                tables: vec!["sys_user".to_string()],
            },
            &[TableInfo {
                name: "sys_user".to_string(),
                module_name: "sys_user".to_string(),
                camel_name: "sysUser".to_string(),
                pascal_name: "SysUser".to_string(),
                comment: None,
                columns: Vec::new(),
                primary_keys: Vec::new(),
            }],
        );

        context.insert("author", "listening").unwrap();
        let value = context.to_value().unwrap();

        assert_eq!(value["author"], json!("listening"));
        assert_eq!(value["project"]["name"], json!("demo"));
        assert_eq!(value["tables"][0]["module_name"], json!("sys_user"));
    }

    #[test]
    fn template_context_rejects_reserved_keys() {
        let mut context = TemplateContext::new(
            &ProjectConfig {
                name: "demo".to_string(),
                output_dir: "./generated".into(),
                overwrite: false,
                modules: Default::default(),
            },
            &DataSourceConfig {
                name: "db1".to_string(),
                kind: DatabaseKind::Mysql,
                url: None,
                host: Some("127.0.0.1".to_string()),
                port: Some(3306),
                database: Some("demo".to_string()),
                schema: None,
                username: Some("root".to_string()),
                password: Some("123456".to_string()),
                tables: vec![],
            },
            &[],
        );

        let error = context.insert("project", "override").unwrap_err();
        assert!(error.to_string().contains("reserved"));
    }
}
