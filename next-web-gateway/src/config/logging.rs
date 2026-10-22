//! Logging configuration.

use serde::Deserialize;

/// Output format of the log stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human readable single line records.
    #[default]
    Text,
    /// One JSON object per line.
    Json,
}

/// Logging and access-log settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    /// Default log level, overridden by `RUST_LOG`.
    #[serde(default = "default_level")]
    pub level: String,
    /// Output format.
    #[serde(default)]
    pub format: LogFormat,
    /// Whether one access log line is emitted per request.
    #[serde(default = "default_true")]
    pub access_log: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_level(),
            format: LogFormat::Text,
            access_log: true,
        }
    }
}

fn default_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}
