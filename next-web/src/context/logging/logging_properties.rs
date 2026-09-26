//! The properties of the logging functionality.
//!
//! The properties are bound from the `next.logging` prefix of the environment,
//! so the command line arguments, the configuration files and the environment
//! variables of the process are all sources of the properties:
//!
//! * `next.logging.level=debug` or `NEXT_LOGGING_LEVEL=debug`
//! * `next.logging.file.path=./logs` or `NEXT_LOGGING_FILE_PATH=./logs`
//! * `next.logging.file.name=/var/log/app/next.log` or
//!   `NEXT_LOGGING_FILE_NAME=/var/log/app/next.log`
//! * `next.logging.file.rotation=daily` or
//!   `NEXT_LOGGING_FILE_ROTATION=daily`

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use next_web_core::env::{BindError, Binder, ConfigurableEnvironment};
use serde::Deserialize;

/// The prefix the properties of the logging functionality are bound from.
pub const LOGGING_PROPERTIES_PREFIX: &str = "next.logging";

/// The stem of the log file that is generated when no file name is configured,
/// which makes a log directory hold the files `next.log` and
/// `next_20260926.log`.
pub const DEFAULT_FILE_STEM: &str = "next";

/// The extension of the log file that is used when the file name of the
/// configuration holds none.
pub const DEFAULT_FILE_SUFFIX: &str = "log";

/// The format of the timestamp of a log record, which is the default of the
/// `next.logging.date-format` property.
pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

/// The directory a file name without a directory resolves to, which is the
/// working directory of the process.
const CURRENT_DIRECTORY: &str = ".";

/// The properties of the logging functionality.
///
/// The properties are bound from the [`LOGGING_PROPERTIES_PREFIX`] of the
/// environment. Every field is optional, and the accessor methods provide the
/// defaults the fields fall back to, so a property that is not configured does
/// not change the default behavior of the logging functionality.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LoggingProperties {
    /// The level of the logging functionality, which is one of `trace`,
    /// `debug`, `info`, `warn` and `error`. Defaults to `info`.
    level: Option<String>,

    /// The levels of the loggers, which are the targets of the records of the
    /// logging functionality, for example `next.logging.levels.sqlx=debug`.
    ///
    /// The level of a logger takes precedence over the level of the logging
    /// functionality, which keeps a logger from writing the records the
    /// application does not need.
    ///
    /// A name that holds no lowercase character is lowered, because the name of
    /// a logger of an environment variable is written in uppercase while the
    /// name of a module of Rust is written in lowercase, so the environment
    /// variable `NEXT_LOGGING_LEVELS_SQLX` configures the logger `sqlx`.
    #[serde(default)]
    levels: HashMap<String, String>,

    /// The directives of the levels of the loggers, which replace the level and
    /// the levels of the loggers when they are configured, for example
    /// `info,sqlx=debug`.
    ///
    /// The directives are the directives of `EnvFilter`, which allows more than
    /// a level of a logger, for example the level of a span of a logger.
    filter: Option<String>,

    /// Whether the log records are written to the console. Defaults to `true`.
    console: Option<bool>,

    /// Whether the log records use ANSI colors. Defaults to using colors only
    /// when no log file is written.
    ansi: Option<bool>,

    /// The format of the timestamp of a log record. Defaults to
    /// [`DEFAULT_DATE_FORMAT`].
    date_format: Option<String>,

    /// The log file the records are written to.
    file: Option<LoggingFileProperties>,
}

impl LoggingProperties {
    /// Returns the properties the given environment describes.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment the properties are bound from.
    ///
    /// # Errors
    ///
    /// Returns [`BindError`] when a value of a property cannot be read as the
    /// type of its field.
    pub fn from_environment(environment: &dyn ConfigurableEnvironment) -> Result<Self, BindError> {
        Binder::new(environment, LOGGING_PROPERTIES_PREFIX).bind()
    }

    /// Returns the name of the configured level, or `None` when the level is
    /// not configured.
    pub fn level_name(&self) -> Option<&str> {
        self.level.as_deref()
    }

    /// Returns the level of the logging functionality, which defaults to `info`
    /// when the level is not configured or when its name is not recognized.
    pub fn level(&self) -> tracing::Level {
        self.level_name()
            .and_then(parse_level)
            .unwrap_or(tracing::Level::INFO)
    }

    /// Returns the levels of the loggers, which is empty when no level of a
    /// logger is configured.
    pub fn levels(&self) -> &HashMap<String, String> {
        &self.levels
    }

    /// Returns the directives of the levels of the loggers.
    ///
    /// The directives are the level of the logging functionality followed by
    /// the level of every logger, which is what `EnvFilter` reads. The
    /// `filter` property replaces both when it is configured.
    ///
    /// # Arguments
    ///
    /// * `warnings` - The warnings of the levels that cannot be read.
    pub fn level_directives(&self, warnings: &mut Vec<String>) -> String {
        if let Some(filter) = self.filter.as_deref() {
            return filter.to_owned();
        }

        let level = self
            .level_name()
            .and_then(parse_level)
            .unwrap_or(tracing::Level::INFO);
        let mut directives = vec![level.to_string().to_lowercase()];

        for (logger, name) in self.levels() {
            if !is_logger_name(logger) {
                warnings.push(format!(
                    "'{logger}' is not the name of a logger, the level of the logger is not read"
                ));

                continue;
            }

            match parse_level(name) {
                Some(level) => directives.push(format!(
                    "{}={}",
                    logger_name(logger),
                    level.to_string().to_lowercase()
                )),
                None => warnings.push(format!(
                    "'{name}' is not the level of the logger '{logger}', using the level of the logging functionality"
                )),
            }
        }

        directives.join(",")
    }

    /// Returns whether the records are written to the console, which defaults
    /// to `true`.
    pub fn console(&self) -> bool {
        self.console.unwrap_or(true)
    }

    /// Returns whether the records use ANSI colors.
    ///
    /// Colors are used only when no log file is written unless the property
    /// configures them, so that the escape sequences of the colors are not
    /// written to a file.
    ///
    /// # Arguments
    ///
    /// * `file_enabled` - Whether a log file is written.
    pub fn ansi(&self, file_enabled: bool) -> bool {
        self.ansi.unwrap_or(!file_enabled)
    }

    /// Returns the format of the timestamp of a log record, which defaults to
    /// [`DEFAULT_DATE_FORMAT`].
    pub fn date_format(&self) -> &str {
        self.date_format.as_deref().unwrap_or(DEFAULT_DATE_FORMAT)
    }

    /// Returns the configuration of the log file, or `None` when no log file is
    /// written.
    ///
    /// The properties of a file that holds no name and no path describe no file
    /// to write, in which case `None` is returned as well.
    pub fn file(&self) -> Option<&LoggingFileProperties> {
        self.file.as_ref().filter(|file| file.is_enabled())
    }
}

/// The properties of the log file.
///
/// The `name` is the name of the log file, which takes precedence over the
/// `path`. The `path` only specifies the directory the log file is stored in,
/// and the name of the file is generated by the system, which is
/// `next_20260926.log` for a rotation and `next.log` when the files are never
/// rotated.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LoggingFileProperties {
    /// The name or the full path of the log file, which takes precedence over
    /// the `path`, for example `next.log`, `./logs/next.log` or
    /// `/var/log/next/next.log`.
    name: Option<String>,

    /// The directory the log file is stored in. The name of the file is
    /// generated by the system, which is `next_20260926.log` for a rotation and
    /// `next.log` when the files are never rotated.
    path: Option<String>,

    /// How often a new log file is started, which is one of `never`,
    /// `minutely`, `hourly`, `daily` and `weekly`. Defaults to `daily`.
    rotation: Option<String>,

    /// The number of the rolled log files that are kept in addition to the file
    /// that is written to. Every file is kept when the property is not
    /// configured.
    max_history: Option<usize>,

    /// The maximum size of a single log file, for example `100MB`.
    ///
    /// A new file of the period is started when the file that is written to
    /// reaches the size, and the name of the file holds the number of the file
    /// of the period, which is `next.2026-09-26.0.log` for example. A single
    /// log file is not limited when the property is not configured.
    max_file_size: Option<String>,

    /// The format of the date that is part of the name of a log file, for
    /// example `%Y%m%d`, which makes the name `next.20260926.log`.
    ///
    /// The date of a period is used when the property is not configured, which
    /// is `%Y-%m-%d` for a daily rotation.
    date_format: Option<String>,

    /// The time zone the name of a log file is dated in, which is `local`,
    /// `utc` or the name of a time zone of the IANA time zone database, for
    /// example `Asia/Shanghai`. Defaults to `local`, which is the time zone of
    /// the process and therefore the time zone of the timestamps of the
    /// records.
    timezone: Option<String>,

    /// The maximum total size of the log files of the directory, for example
    /// `1GB`. The oldest files are removed until the files fit the size, and
    /// the file that is written to is never removed. Every file is kept when
    /// the property is not configured.
    total_size_cap: Option<String>,

    /// Whether a new log file is started when the application starts, which
    /// defaults to `false`, so the records of a new application are added to
    /// the file of the period it starts in.
    ///
    /// The name of a file holds the number of the file of the period as soon as
    /// the property is enabled, so the files of the application that started
    /// earlier are kept.
    roll_on_start: Option<bool>,

    /// Whether the log files that already exist are cleaned when the
    /// application starts. Defaults to `false`.
    clean_history_on_start: Option<bool>,
}

impl LoggingFileProperties {
    /// Returns whether a log file is configured.
    ///
    /// A log file is written only when a `name` or a `path` is configured.
    pub fn is_enabled(&self) -> bool {
        self.name.is_some() || self.path.is_some()
    }

    /// Returns the directory the log file is stored in.
    ///
    /// The directory is determined in the following order:
    ///
    /// 1. The directory of the `name`, which is `/var/log/app` for
    ///    `/var/log/app/next.log`.
    /// 2. The `path`.
    /// 3. The working directory of the process, which is used when only a name
    ///    of a file is configured.
    pub fn directory(&self) -> PathBuf {
        if let Some(name) = self.name.as_deref().and_then(parent_of) {
            return name;
        }

        self.path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(CURRENT_DIRECTORY))
    }

    /// Returns the stem of the log file, which is `next` for `next.log`.
    ///
    /// The date of a rotated file is written after the stem, so the records of
    /// a `next.log` file are written to `next_20260926.log`, and the number of
    /// the file of a period is written after the date, which is
    /// `next_20260926_0.log`.
    pub fn stem(&self) -> String {
        self.name
            .as_deref()
            .and_then(file_stem)
            .unwrap_or_else(|| DEFAULT_FILE_STEM.to_owned())
    }

    /// Returns the extension of the log file, which defaults to `log`.
    pub fn suffix(&self) -> String {
        self.name
            .as_deref()
            .and_then(file_suffix)
            .unwrap_or_else(|| DEFAULT_FILE_SUFFIX.to_owned())
    }

    /// Returns how often a new log file is started, which defaults to
    /// [`LogRotation::Daily`].
    pub fn rotation(&self) -> LogRotation {
        self.rotation
            .as_deref()
            .and_then(LogRotation::parse)
            .unwrap_or_default()
    }

    /// Returns the number of the split log files that are kept, or `None` when
    /// the property is not configured.
    pub fn max_history(&self) -> Option<usize> {
        self.max_history
    }

    /// Returns the maximum size of a single log file, or `None` when the
    /// property is not configured.
    pub fn max_file_size(&self) -> Option<&str> {
        self.max_file_size.as_deref()
    }

    /// Returns the format of the date that is part of the name of a log file,
    /// or `None` when the date of the period is used.
    pub fn file_date_format(&self) -> Option<&str> {
        self.date_format.as_deref()
    }

    /// Returns the name of the time zone the name of a log file is dated in, or
    /// `None` when the time zone of the process is used.
    pub fn timezone(&self) -> Option<&str> {
        self.timezone.as_deref()
    }

    /// Returns the maximum total size of the log files, or `None` when the
    /// property is not configured.
    pub fn total_size_cap(&self) -> Option<&str> {
        self.total_size_cap.as_deref()
    }

    /// Returns whether a new log file is started when the application starts,
    /// which defaults to `false`.
    pub fn roll_on_start(&self) -> bool {
        self.roll_on_start.unwrap_or(false)
    }

    /// Returns whether the log files that already exist are cleaned when the
    /// application starts, which defaults to `false`.
    pub fn clean_history_on_start(&self) -> bool {
        self.clean_history_on_start.unwrap_or(false)
    }
}

/// How often a new log file is started.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogRotation {
    /// Every record is written to the same file.
    Never,

    /// A new file is started every minute.
    Minutely,

    /// A new file is started every hour.
    Hourly,

    /// A new file is started every day, which is the default.
    #[default]
    Daily,

    /// A new file is started every week, which starts on Monday.
    Weekly,
}

impl LogRotation {
    /// Returns the rotation of the given name, or `None` when the name is not
    /// recognized.
    ///
    /// # Arguments
    ///
    /// * `value` - The name of the rotation.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "never" | "none" | "off" => Some(Self::Never),
            "minutely" | "minute" => Some(Self::Minutely),
            "hourly" | "hour" => Some(Self::Hourly),
            "daily" | "day" => Some(Self::Daily),
            "weekly" | "week" => Some(Self::Weekly),
            _ => None,
        }
    }
}

/// Returns the level of the given name, or `None` when the name is not
/// recognized.
///
/// The name is not case sensitive, and `warning` is equivalent to `warn`.
///
/// # Arguments
///
/// * `level` - The name of the level.
pub fn parse_level(level: &str) -> Option<tracing::Level> {
    match level.trim().to_ascii_lowercase().as_str() {
        "trace" => Some(tracing::Level::TRACE),
        "debug" => Some(tracing::Level::DEBUG),
        "info" => Some(tracing::Level::INFO),
        "warn" | "warning" => Some(tracing::Level::WARN),
        "error" => Some(tracing::Level::ERROR),
        _ => None,
    }
}

/// Returns whether the given name is the name of a logger the level of which
/// can be configured.
///
/// The levels of the loggers are the directives of an `EnvFilter`, so a name
/// that holds a character which takes part in the syntax of a directive is not
/// the name of a logger.
///
/// # Arguments
///
/// * `name` - The name of the logger.
fn is_logger_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | ':' | '-')
        })
}

/// Returns the name of a logger a directive is written with.
///
/// The name of a logger of an environment variable is written in uppercase,
/// because that is how the name of an environment variable is written, while
/// the name of a module of Rust is written in lowercase, so a name that holds
/// no lowercase character is lowered.
///
/// # Arguments
///
/// * `name` - The name of the logger.
fn logger_name(name: &str) -> String {
    if name.chars().any(|character| character.is_ascii_lowercase()) {
        return name.to_owned();
    }

    name.to_ascii_lowercase()
}

/// Returns the directory of the given name, or `None` when it holds none.
///
/// # Arguments
///
/// * `name` - The name or the path of the log file.
fn parent_of(name: &str) -> Option<PathBuf> {
    Path::new(name)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

/// Returns the stem of the given name, or `None` when it holds none.
///
/// # Arguments
///
/// * `name` - The name or the path of the log file.
fn file_stem(name: &str) -> Option<String> {
    Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(ToOwned::to_owned)
}

/// Returns the extension of the given name, or `None` when it holds none.
///
/// # Arguments
///
/// * `name` - The name or the path of the log file.
fn file_suffix(name: &str) -> Option<String> {
    Path::new(name)
        .extension()
        .and_then(|suffix| suffix.to_str())
        .filter(|suffix| !suffix.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use next_web_core::env::{BaseEnvironment, MapPropertySource};
    use next_web_core::util::indexmap::IndexMap;

    use super::*;

    /// Creates an environment holding the given properties.
    fn environment(properties: &[(&str, &str)]) -> BaseEnvironment {
        let mut environment = BaseEnvironment::new();
        environment
            .property_sources()
            .add_last(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                    .collect::<IndexMap<_, _>>(),
            )));

        environment
    }

    /// Binds the properties of the given properties.
    fn bind(properties: &[(&str, &str)]) -> LoggingProperties {
        LoggingProperties::from_environment(&environment(properties)).unwrap()
    }

    #[test]
    fn binds_the_properties_from_environment_variable_names() {
        let properties = bind(&[
            ("NEXT_LOGGING_LEVEL", "debug"),
            ("NEXT_LOGGING_FILE_NAME", "next.log"),
            ("NEXT_LOGGING_FILE_PATH", "./logs"),
            ("NEXT_LOGGING_FILE_MAX_HISTORY", "7"),
        ]);

        assert_eq!(properties.level_name(), Some("debug"));
        assert_eq!(properties.level(), tracing::Level::DEBUG);

        let file = properties.file().unwrap();
        assert_eq!(file.stem(), "next");
        assert_eq!(file.suffix(), "log");
        assert_eq!(file.directory(), PathBuf::from("./logs"));
        assert_eq!(file.max_history(), Some(7));
    }

    #[test]
    fn binds_the_properties_of_a_configuration_file() {
        let properties = bind(&[
            ("next.logging.level", "warn"),
            ("next.logging.date-format", "%H:%M:%S"),
            ("next.logging.file.path", "/var/log/next"),
            ("next.logging.file.rotation", "hourly"),
            ("next.logging.file.clean-history-on-start", "true"),
        ]);

        assert_eq!(properties.level(), tracing::Level::WARN);
        assert_eq!(properties.date_format(), "%H:%M:%S");

        let file = properties.file().unwrap();
        assert_eq!(file.directory(), PathBuf::from("/var/log/next"));
        assert_eq!(file.rotation(), LogRotation::Hourly);
        assert!(file.clean_history_on_start());
    }

    #[test]
    fn defaults_the_missing_properties() {
        let properties = bind(&[]);

        assert_eq!(properties.level(), tracing::Level::INFO);
        assert!(properties.console());
        assert!(properties.ansi(false));
        assert_eq!(properties.date_format(), DEFAULT_DATE_FORMAT);
        assert!(properties.file().is_none());
    }

    #[test]
    fn defaults_the_level_of_an_unknown_name() {
        assert_eq!(parse_level("verbose"), None);
        assert_eq!(
            bind(&[("next.logging.level", "verbose")]).level(),
            tracing::Level::INFO
        );
    }

    #[test]
    fn parses_the_levels() {
        assert_eq!(parse_level("TRACE"), Some(tracing::Level::TRACE));
        assert_eq!(parse_level(" Debug "), Some(tracing::Level::DEBUG));
        assert_eq!(parse_level("info"), Some(tracing::Level::INFO));
        assert_eq!(parse_level("warn"), Some(tracing::Level::WARN));
        assert_eq!(parse_level("warning"), Some(tracing::Level::WARN));
        assert_eq!(parse_level("error"), Some(tracing::Level::ERROR));
    }

    #[test]
    fn binds_the_levels_of_the_loggers() {
        let properties = bind(&[
            ("next.logging.level", "info"),
            ("next.logging.levels.sqlx", "warn"),
            ("next.logging.levels.next_web::context", "debug"),
        ]);

        assert_eq!(
            properties.levels().get("sqlx").map(String::as_str),
            Some("warn")
        );
        assert_eq!(
            properties
                .levels()
                .get("next_web::context")
                .map(String::as_str),
            Some("debug")
        );
    }

    #[test]
    fn binds_the_levels_of_the_loggers_from_environment_variable_names() {
        let properties = bind(&[
            ("NEXT_LOGGING_LEVEL", "info"),
            ("NEXT_LOGGING_LEVELS_SQLX", "warn"),
        ]);
        let mut warnings = Vec::new();

        assert_eq!(
            properties.levels().get("SQLX").map(String::as_str),
            Some("warn")
        );
        // The name of the logger of an environment variable is written in
        // uppercase, which the directive lowers.
        assert_eq!(properties.level_directives(&mut warnings), "info,sqlx=warn");
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn reads_the_directives_of_the_levels_of_the_loggers() {
        let properties = bind(&[
            ("next.logging.level", "info"),
            ("next.logging.levels.sqlx", "warn"),
            ("next.logging.levels.next_web", "debug"),
        ]);
        let mut warnings = Vec::new();
        let directives = properties.level_directives(&mut warnings);

        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        assert!(directives.starts_with("info,"), "unexpected: {directives}");
        assert!(directives.contains("sqlx=warn"), "unexpected: {directives}");
        assert!(
            directives.contains("next_web=debug"),
            "unexpected: {directives}"
        );
    }

    #[test]
    fn uses_the_level_of_the_logging_functionality_when_no_logger_is_configured() {
        let properties = bind(&[("next.logging.level", "WARN")]);
        let mut warnings = Vec::new();

        assert_eq!(properties.level_directives(&mut warnings), "warn");
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn reads_the_filter_instead_of_the_levels_of_the_loggers() {
        let properties = bind(&[
            ("next.logging.level", "info"),
            ("next.logging.filter", "error,next_web[request]=trace"),
            ("next.logging.levels.sqlx", "warn"),
        ]);
        let mut warnings = Vec::new();

        assert_eq!(
            properties.level_directives(&mut warnings),
            "error,next_web[request]=trace"
        );
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn reports_the_levels_of_the_loggers_that_cannot_be_read() {
        let properties = bind(&[
            ("next.logging.level", "info"),
            ("next.logging.levels.sqlx", "verbose"),
            ("next.logging.levels.logger=with=equals", "warn"),
        ]);
        let mut warnings = Vec::new();
        let directives = properties.level_directives(&mut warnings);

        assert_eq!(directives, "info");
        assert_eq!(warnings.len(), 2, "unexpected warnings: {warnings:?}");
        assert!(warnings.iter().any(|warning| warning.contains("verbose")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("logger=with=equals")));
    }

    #[test]
    fn generates_the_log_file_of_a_directory() {
        let properties = bind(&[("next.logging.file.path", "./logs")]);
        let file = properties.file().unwrap();

        assert_eq!(file.directory(), PathBuf::from("./logs"));
        assert_eq!(file.stem(), DEFAULT_FILE_STEM);
        assert_eq!(file.suffix(), DEFAULT_FILE_SUFFIX);
        assert_eq!(file.rotation(), LogRotation::Daily);
    }

    #[test]
    fn prefers_the_name_over_the_path() {
        let properties = bind(&[
            ("next.logging.file.path", "./logs"),
            ("next.logging.file.name", "/var/log/app/next.log"),
        ]);
        let file = properties.file().unwrap();

        assert_eq!(file.directory(), PathBuf::from("/var/log/app"));
        assert_eq!(file.stem(), "next");
        assert_eq!(file.suffix(), "log");
    }

    #[test]
    fn resolves_the_directory_of_a_name_that_has_none() {
        let properties = bind(&[
            ("next.logging.file.path", "./logs"),
            ("next.logging.file.name", "application.log"),
        ]);
        let file = properties.file().unwrap();

        assert_eq!(file.directory(), PathBuf::from("./logs"));
        assert_eq!(file.stem(), "application");
    }

    #[test]
    fn resolves_the_directory_of_a_name_without_a_path() {
        let properties = bind(&[("next.logging.file.name", "application.log")]);
        let file = properties.file().unwrap();

        assert_eq!(file.directory(), PathBuf::from(CURRENT_DIRECTORY));
        assert_eq!(file.stem(), "application");
    }

    #[test]
    fn parses_the_rotations() {
        assert_eq!(LogRotation::parse("never"), Some(LogRotation::Never));
        assert_eq!(LogRotation::parse("MINUTELY"), Some(LogRotation::Minutely));
        assert_eq!(LogRotation::parse("hourly"), Some(LogRotation::Hourly));
        assert_eq!(LogRotation::parse("daily"), Some(LogRotation::Daily));
        assert_eq!(LogRotation::parse("weekly"), Some(LogRotation::Weekly));
        assert_eq!(LogRotation::parse("monthly"), None);
    }

    #[test]
    fn defaults_the_rotation_of_an_unknown_name() {
        let properties = bind(&[
            ("next.logging.file.path", "./logs"),
            ("next.logging.file.rotation", "monthly"),
        ]);

        assert_eq!(properties.file().unwrap().rotation(), LogRotation::Daily);
    }

    #[test]
    fn binds_the_rolling_properties_from_environment_variable_names() {
        let properties = bind(&[
            ("NEXT_LOGGING_FILE_PATH", "./logs"),
            ("NEXT_LOGGING_FILE_MAX_FILE_SIZE", "10MB"),
            ("NEXT_LOGGING_FILE_TOTAL_SIZE_CAP", "1GB"),
            ("NEXT_LOGGING_FILE_DATE_FORMAT", "%Y%m%d"),
            ("NEXT_LOGGING_FILE_TIMEZONE", "Asia/Shanghai"),
            ("NEXT_LOGGING_FILE_ROLL_ON_START", "true"),
            ("NEXT_LOGGING_FILE_ROTATION", "weekly"),
            ("NEXT_LOGGING_FILE_MAX_HISTORY", "7"),
        ]);
        let file = properties.file().unwrap();

        assert_eq!(file.max_file_size(), Some("10MB"));
        assert_eq!(file.total_size_cap(), Some("1GB"));
        assert_eq!(file.file_date_format(), Some("%Y%m%d"));
        assert_eq!(file.timezone(), Some("Asia/Shanghai"));
        assert!(file.roll_on_start());
        assert_eq!(file.rotation(), LogRotation::Weekly);
        assert_eq!(file.max_history(), Some(7));
    }

    #[test]
    fn defaults_the_rolling_properties() {
        let properties = bind(&[("next.logging.file.path", "./logs")]);
        let file = properties.file().unwrap();

        assert_eq!(file.max_file_size(), None);
        assert_eq!(file.total_size_cap(), None);
        assert_eq!(file.file_date_format(), None);
        assert_eq!(file.timezone(), None);
        assert!(!file.roll_on_start());
    }

    #[test]
    fn ignores_the_file_properties_that_hold_no_file() {
        let properties = bind(&[("next.logging.file.rotation", "hourly")]);

        assert!(properties.file().is_none());
    }

    #[test]
    fn reports_the_file_that_holds_the_size_of_a_log_file() {
        let properties = bind(&[
            ("next.logging.file.path", "./logs"),
            ("next.logging.file.max-file-size", "100MB"),
        ]);

        assert_eq!(properties.file().unwrap().max_file_size(), Some("100MB"));
    }
}
