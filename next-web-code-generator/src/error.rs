use std::path::PathBuf;

/// Result type used by the code generator crate.
pub type Result<T> = std::result::Result<T, CodegenError>;

/// Error type used by configuration loading, database introspection and code generation.
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    /// Wraps a filesystem error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Wraps a `toml` deserialization error.
    #[error("toml deserialize error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    /// Wraps a `toml` serialization error.
    #[error("toml serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// Wraps a database driver error.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Wraps a JSON serialization or deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Returned when the requested config file cannot be found.
    #[error("configuration file does not exist: {0}")]
    MissingConfig(PathBuf),

    /// Returned when a named datasource cannot be found.
    #[error("datasource '{0}' was not found in the generator configuration")]
    MissingDatasource(String),

    /// Returned when a template has no defined source.
    #[error("template '{0}' must define either `path` or `inline`")]
    InvalidTemplateSource(String),

    /// Returned when a template defines both an inline and file source.
    #[error("template '{0}' defines both `path` and `inline`, only one source is allowed")]
    ConflictingTemplateSource(String),

    /// Returned when a database kind is unavailable in the current build.
    #[error("unsupported database kind in the current build: {0}")]
    UnsupportedDatabase(String),

    /// Returned when template parsing or rendering fails.
    #[error("unsupported template syntax: {0}")]
    Template(String),

    /// Returned when attempting to overwrite a reserved template context key.
    #[error("template context key '{0}' is reserved and cannot be overridden")]
    ReservedTemplateKey(String),

    /// Returned when an invalid template context key is provided.
    #[error("invalid template context key: {0}")]
    InvalidTemplateKey(String),

    /// Returned when a required configuration field is absent.
    #[error("missing required field: {0}")]
    MissingField(String),
}
