use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CodegenError, Result};
use crate::model::DatabaseKind;

/// Top-level generator configuration loaded from a `toml` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Generation target information.
    pub project: ProjectConfig,
    /// Database sources used to introspect tables.
    pub datasources: Vec<DataSourceConfig>,
    /// Template definitions used to generate output files.
    pub templates: Vec<TemplateConfig>,
}

impl GeneratorConfig {
    /// Load a generator configuration from a `toml` file.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(CodegenError::MissingConfig(path.to_path_buf()));
        }

        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Save the configuration to a `toml` file.
    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Find a datasource by name or return the first configured datasource.
    pub fn datasource(&self, name: Option<&str>) -> Result<&DataSourceConfig> {
        match name {
            Some(name) => self
                .datasources
                .iter()
                .find(|datasource| datasource.name == name)
                .ok_or_else(|| CodegenError::MissingDatasource(name.to_string())),
            None => self
                .datasources
                .first()
                .ok_or_else(|| CodegenError::MissingField("datasources".to_string())),
        }
    }

    /// Build an example configuration file.
    pub fn example() -> &'static str {
        r#"# next-web-code-generator example configuration

[project]
name = "sample-generated-app"
output_dir = "./generated/sample-generated-app"
overwrite = false

[project.modules]
create_mod_rs = true
append_pub_mod = true

[[datasources]]
name = "db1"
kind = "mysql"
host = "127.0.0.1"
port = 3306
database = "demo"
username = "root"
password = "123456"
tables = ["sys_user", "sys_role"]

[[templates]]
name = "lib"
path = "./templates/src_lib.rs.tpl"
output = "src/lib.rs"
per_table = false
overwrite = true

[[templates]]
name = "models_mod"
path = "./templates/models_mod.rs.tpl"
output = "src/models/mod.rs"
per_table = false
overwrite = true

[[templates]]
name = "entity"
path = "./templates/entity.rs.tpl"
output = "src/models/{{table.module_name}}.rs"
per_table = true
"#
    }
}

/// Generation target options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Logical module namespace or generation target name.
    pub name: String,
    /// Output root directory for generated mapping code.
    pub output_dir: PathBuf,
    /// Whether existing generated files may be overwritten.
    #[serde(default)]
    pub overwrite: bool,
    /// Module maintenance options for generated `.rs` files and folders.
    #[serde(default)]
    pub modules: ModuleConfig,
}

/// Controls how generated Rust module declarations are maintained on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleConfig {
    /// Whether to create missing `mod.rs` files for generated module directories.
    #[serde(default)]
    pub create_mod_rs: bool,
    /// Whether to append missing `pub mod xxx;` declarations into managed `mod.rs` files.
    #[serde(default)]
    pub append_pub_mod: bool,
}

/// Datasource definition used by metadata introspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    /// Logical datasource name referenced by CLI flags.
    pub name: String,
    /// Database kind.
    pub kind: DatabaseKind,
    /// Optional full connection URL.
    pub url: Option<String>,
    /// Database host.
    pub host: Option<String>,
    /// Database port.
    pub port: Option<u16>,
    /// Database name.
    pub database: Option<String>,
    /// Database schema for PostgreSQL or explicit schema override.
    pub schema: Option<String>,
    /// Login username.
    pub username: Option<String>,
    /// Login password.
    pub password: Option<String>,
    /// Tables that should be generated. Empty means all tables in the schema.
    #[serde(default)]
    pub tables: Vec<String>,
}

impl DataSourceConfig {
    /// Build a connection URL from either `url` or discrete connection fields.
    pub fn connection_url(&self) -> Result<String> {
        if let Some(url) = &self.url {
            return Ok(url.clone());
        }

        let host = self.host.as_deref().ok_or_else(|| {
            CodegenError::MissingField(format!("datasource '{}'.host", self.name))
        })?;
        let database = self.database.as_deref().ok_or_else(|| {
            CodegenError::MissingField(format!("datasource '{}'.database", self.name))
        })?;

        let username = self.username.as_deref().unwrap_or(match self.kind {
            DatabaseKind::Mysql => "root",
            DatabaseKind::Postgres => "postgres",
        });
        let password = self.password.as_deref().unwrap_or_default();

        let url = match self.kind {
            DatabaseKind::Mysql => format!(
                "mysql://{username}:{password}@{host}:{}/{}",
                self.port.unwrap_or(3306),
                database
            ),
            DatabaseKind::Postgres => format!(
                "postgres://{username}:{password}@{host}:{}/{}",
                self.port.unwrap_or(5432),
                database
            ),
        };

        Ok(url)
    }

    /// Return the schema used for metadata queries.
    pub fn schema_name(&self) -> Result<String> {
        if let Some(schema) = &self.schema {
            return Ok(schema.clone());
        }

        match self.kind {
            DatabaseKind::Mysql => self.database.clone().ok_or_else(|| {
                CodegenError::MissingField(format!("datasource '{}'.database", self.name))
            }),
            DatabaseKind::Postgres => Ok("public".to_string()),
        }
    }
}

/// Template definition used to render one or more files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Template logical name.
    pub name: String,
    /// Template file path, relative to the config file when not absolute.
    pub path: Option<PathBuf>,
    /// Inline template content.
    pub inline: Option<String>,
    /// Output path template, relative to the generated project root.
    pub output: String,
    /// Whether the template should be rendered once per table.
    #[serde(default)]
    pub per_table: bool,
    /// Override the project-level overwrite policy for this template.
    pub overwrite: Option<bool>,
}

impl TemplateConfig {
    /// Resolve the template content source and validate the definition.
    pub fn read_template(&self, base_dir: &Path) -> Result<String> {
        match (&self.path, &self.inline) {
            (Some(_), Some(_)) => Err(CodegenError::ConflictingTemplateSource(self.name.clone())),
            (None, None) => Err(CodegenError::InvalidTemplateSource(self.name.clone())),
            (None, Some(inline)) => Ok(inline.clone()),
            (Some(path), None) => {
                let path = if path.is_absolute() {
                    path.clone()
                } else {
                    base_dir.join(path)
                };
                Ok(std::fs::read_to_string(path)?)
            }
        }
    }

    /// Resolve whether this template may overwrite an existing file.
    pub fn overwrite_enabled(&self, project: &ProjectConfig) -> bool {
        self.overwrite.unwrap_or(project.overwrite)
    }
}
