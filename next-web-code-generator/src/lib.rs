//! Code generation core for database-driven Rust mapping code generation.
//!
//! This crate provides:
//! - a `toml` based configuration model
//! - database table introspection for MySQL and PostgreSQL
//! - a lightweight customizable template engine
//! - CLI commands that initialize config files, inspect metadata and generate code
//!
//! PostgreSQL introspection is implemented behind the `postgres` cargo feature.

/// CLI types and command execution helpers.
pub mod cli;
/// `toml` configuration structures and file helpers.
pub mod config;
/// Strongly typed template context and extension points.
pub mod context;
/// Database metadata loading for supported drivers.
pub mod database;
/// Shared crate error types.
pub mod error;
/// High-level generation orchestration.
pub mod generator;
/// Shared metadata and naming models.
pub mod model;
/// Lightweight template rendering support.
pub mod template;

pub use cli::{Cli, CliCommand, run_cli};
pub use config::{DataSourceConfig, GeneratorConfig, ModuleConfig, ProjectConfig, TemplateConfig};
pub use context::{TemplateContext, TemplateProjectContext, TemplateTimeContext};
pub use database::DatabaseIntrospector;
pub use error::{CodegenError, Result};
pub use generator::{GeneratedFile, Generator};
pub use model::{ColumnInfo, DatabaseKind, TableInfo};
pub use template::{
    TemplateEngine, TemplateHelper, TemplateHelperRegistry, render_string_template,
    render_string_template_with_helpers,
};
