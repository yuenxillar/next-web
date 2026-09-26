//! The event handler that initializes the logging functionality.
//!
//! The handler binds the properties of the logging functionality from the
//! `next.logging` prefix of the environment once the environment is prepared.

use next_web_core::env::ConfigurableEnvironment;
use tracing_appender::non_blocking::{ErrorCounter, NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::fmt::time::ChronoLocal;
use tracing_subscriber::fmt::writer::{BoxMakeWriter, Tee};
use tracing_subscriber::EnvFilter;

use crate::context::logging::logging_properties::{
    parse_level, LoggingFileProperties, LoggingProperties,
};
use crate::context::logging::logging_rolling::{RollingFileWriter, RollingPolicy};
use crate::context::logging::logging_warnings::WarningReporter;
use crate::{ApplicationEventHandler, Event};

/// An [`ApplicationEventHandler`] that initializes the logging functionality.
///
/// The handler binds the [`LoggingProperties`] from the `next.logging` prefix of
/// the environment once the environment is prepared, and installs the global
/// subscriber of the logging functionality from them:
///
/// * `next.logging.level` / `NEXT_LOGGING_LEVEL` decides the level;
/// * `next.logging.console` decides whether the records are written to the
///   console, which is what happens by default;
/// * `next.logging.date-format` decides the format of the timestamp of a
///   record;
/// * `next.logging.file.name` / `NEXT_LOGGING_FILE_NAME` decides the log file,
///   which takes precedence over `next.logging.file.path`, the directory the
///   records are written to under the files the policy of the file names, which
///   are `next_20260926.log` and `next.log`;
/// * `next.logging.file.rotation` / `NEXT_LOGGING_FILE_ROTATION` decides how
///   often a new file is started, `next.logging.file.max-file-size` decides the
///   size a file is split at, and `next.logging.file.max-history` decides the
///   number of the rolled files that are kept;
/// * `next.logging.file.date-format` and `next.logging.file.timezone` decide the
///   date of the names of the files, which is the date of the process by
///   default, and `next.logging.file.total-size-cap`,
///   `next.logging.file.roll-on-start` and
///   `next.logging.file.clean-history-on-start` decide which of the files of the
///   directory are kept.
///
/// The subscriber of the logging functionality is installed once, so the
/// handler does not install it a second time when one is already installed. The
/// properties fall back to their defaults when they cannot be bound, so the
/// configuration of the logging functionality never stops an application from
/// starting.
#[derive(Default)]
pub struct LoggingEventHandler {
    /// The guard of the thread that writes the records to the log file.
    ///
    /// The records are written in a non blocking way, and dropping the guard
    /// drops the records that have not been written to the file yet, so the
    /// guard lives as long as the handler.
    file_guard: Option<WorkerGuard>,

    /// The number of the records the logging functionality dropped.
    ///
    /// The records of the logging functionality are written from a thread of
    /// its own, which drops the records it cannot keep up with, so the count is
    /// reported when the handler is dropped.
    dropped_records: Option<ErrorCounter>,
}

impl LoggingEventHandler {
    /// Returns the guard of the thread that writes the records to the log file,
    /// or `None` when no log file is written.
    pub fn file_guard(&self) -> Option<&WorkerGuard> {
        self.file_guard.as_ref()
    }

    /// Binds the properties of the logging functionality and initializes it.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment the properties are bound from.
    fn on_environment_prepared(&mut self, environment: &dyn ConfigurableEnvironment) {
        // The warnings are reported once the subscriber is installed, because
        // that is the only way a warning about the configuration of the logging
        // functionality reaches the console and the log file.
        let mut warnings = Vec::new();

        let properties = match LoggingProperties::from_environment(environment) {
            Ok(properties) => properties,
            Err(error) => {
                warnings.push(format!(
                    "failed to bind the logging properties, using the defaults: {error}"
                ));

                LoggingProperties::default()
            }
        };

        if let Some(level) = properties.level_name() {
            if parse_level(level).is_none() {
                warnings.push(format!("unknown log level '{level}', using info"));
            }
        }

        let filter = Self::filter(&properties, &mut warnings);
        let file = properties.file();
        let console = properties.console();
        // An application that writes its records neither to the console nor to
        // a file does not need a subscriber.
        let (writer, guard, dropped_records) = match Self::writer(console, file, &mut warnings) {
            Some(writer) => writer,
            None => return,
        };

        let format = tracing_subscriber::fmt::format()
            .with_timer(ChronoLocal::new(properties.date_format().to_owned()))
            .with_level(true)
            .with_target(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_source_location(true)
            .with_thread_names(true);

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(properties.ansi(file.is_some()))
            .with_writer(writer)
            .event_format(format);

        match subscriber.try_init() {
            Ok(()) => {
                self.file_guard = guard;
                self.dropped_records = dropped_records;

                for warning in warnings {
                    tracing::warn!("{warning}");
                }
            }
            // Nothing is logged without a subscriber, so the failure to install
            // one is the only report that is written to the standard error
            // stream.
            Err(error) => {
                eprintln!("[next-web] failed to initialize the logging functionality: {error}")
            }
        }
    }

    /// Returns the filter of the levels of the loggers.
    ///
    /// The filter is built from the level of the logging functionality and the
    /// levels of the loggers, and the directives of the `filter` property
    /// replace both of them. The level of the logging functionality is used
    /// when the directives cannot be read.
    ///
    /// # Arguments
    ///
    /// * `properties` - The properties of the logging functionality.
    /// * `warnings` - The warnings of the levels that cannot be read.
    fn filter(properties: &LoggingProperties, warnings: &mut Vec<String>) -> EnvFilter {
        let directives = properties.level_directives(warnings);

        match EnvFilter::builder().parse(&directives) {
            Ok(filter) => filter,
            Err(error) => {
                warnings.push(format!(
                    "the levels of the loggers '{directives}' cannot be read: {error}, using the level of the logging functionality"
                ));

                EnvFilter::new(properties.level().to_string().to_lowercase())
            }
        }
    }

    /// Returns the destination the records are written to and the guard of the
    /// thread that writes them to the log file, or `None` when there is no
    /// destination.
    ///
    /// The records are written to the console and to the log file at the same
    /// time; the destination of the records that are written to a file only is
    /// the file itself.
    ///
    /// # Arguments
    ///
    /// * `console` - Whether the records are written to the console.
    /// * `file` - The configuration of the log file.
    /// * `warnings` - The warnings of the configuration, which are reported
    ///   once the subscriber is installed.
    ///
    /// # Returns
    ///
    /// Returns the destination of the records, the guard of the thread that
    /// writes them to the log file, and the number of the records the logging
    /// functionality dropped.
    fn writer(
        console: bool,
        file: Option<&LoggingFileProperties>,
        warnings: &mut Vec<String>,
    ) -> Option<(BoxMakeWriter, Option<WorkerGuard>, Option<ErrorCounter>)> {
        let Some(file) = file else {
            return console
                .then(Self::console_writer)
                .map(|writer| (writer, None, None));
        };

        match Self::file_writer(file, warnings) {
            Ok((file_writer, guard, dropped_records)) => {
                let writer = if console {
                    BoxMakeWriter::new(Tee::new(std::io::stdout, file_writer))
                } else {
                    BoxMakeWriter::new(file_writer)
                };

                Some((writer, Some(guard), Some(dropped_records)))
            }
            Err(error) => {
                warnings.push(error);

                console
                    .then(Self::console_writer)
                    .map(|writer| (writer, None, None))
            }
        }
    }

    /// Returns the destination of the records that are written to the console.
    fn console_writer() -> BoxMakeWriter {
        BoxMakeWriter::new(std::io::stdout)
    }

    /// Creates the destination of the records that are written to the log file
    /// and the guard of the thread that writes them.
    ///
    /// Returns the failure to create the file as its error, in which case the
    /// records fall back to the console.
    ///
    /// # Arguments
    ///
    /// * `file` - The configuration of the log file.
    /// * `warnings` - The warnings of the configuration, which are reported
    ///   once the subscriber is installed.
    fn file_writer(
        file: &LoggingFileProperties,
        warnings: &mut Vec<String>,
    ) -> Result<(NonBlocking, WorkerGuard, ErrorCounter), String> {
        let policy = RollingPolicy::resolve(file, warnings);
        let directory = policy.directory().to_path_buf();
        let writer =
            RollingFileWriter::new(policy, WarningReporter::logging()).map_err(|error| {
                format!(
                    "failed to create the log file of {}: {error}",
                    directory.display()
                )
            })?;
        let (writer, guard) = NonBlockingBuilder::default().finish(writer);
        let dropped_records = writer.error_counter();

        Ok((writer, guard, dropped_records))
    }
}

impl ApplicationEventHandler for LoggingEventHandler {
    fn handle_event(&mut self, event: crate::Event) {
        // The properties of the logging functionality are only complete once
        // the environment is prepared, because the command line arguments, the
        // configuration files and the environment variables have been added to
        // it by then, which is what the level and the file are configured with.
        if let Event::EnvironmentPrepared(payload) = event {
            self.on_environment_prepared(payload.environment);
        }
    }
}

impl Drop for LoggingEventHandler {
    /// Reports the records the logging functionality dropped.
    ///
    /// The report is written before the guard of the thread that writes the
    /// records is dropped, because the records of the logging functionality are
    /// only written to the console and to the log file as long as that thread
    /// is running.
    fn drop(&mut self) {
        let dropped = self
            .dropped_records
            .as_ref()
            .map(ErrorCounter::dropped_lines)
            .unwrap_or(0);

        let Some(warning) = dropped_records_warning(dropped) else {
            return;
        };

        tracing::warn!(target: "next_web::logging", "{warning}");
    }
}

/// Returns the report of the records the logging functionality dropped, or
/// `None` when no record was dropped.
///
/// # Arguments
///
/// * `dropped` - The number of the records the logging functionality dropped.
fn dropped_records_warning(dropped: usize) -> Option<String> {
    if dropped == 0 {
        return None;
    }

    Some(format!(
        "the logging functionality dropped {dropped} records, the records of the application are written faster than the records of the log file"
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::path::{Path, PathBuf};

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

    /// Creates an empty directory the test owns.
    fn directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir()
            .join("next-web-logging-tests")
            .join(format!("{}-{name}", std::process::id()));

        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    /// Returns the names of the files of the given directory.
    fn file_names(directory: &Path) -> Vec<String> {
        let mut names = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();

        names
    }

    /// Writes a record and waits for the thread that writes the files to write
    /// it.
    fn write_record(mut writer: NonBlocking, guard: WorkerGuard) {
        writer.write_all(b"a log record\n").unwrap();
        writer.flush().unwrap();

        drop(writer);
        drop(guard);
    }

    #[test]
    fn writes_the_generated_file_name_of_a_directory() {
        let directory = directory("generates-the-file-name");
        let properties = LoggingProperties::from_environment(&environment(&[(
            "NEXT_LOGGING_FILE_PATH",
            directory.to_str().unwrap(),
        )]))
        .unwrap();

        let file = properties.file().unwrap();
        let (writer, guard, _dropped) =
            LoggingEventHandler::file_writer(file, &mut Vec::new()).unwrap();
        write_record(writer, guard);

        let names = file_names(&directory);
        assert_eq!(names.len(), 1, "unexpected files: {names:?}");
        assert!(names[0].starts_with("next_"), "unexpected name: {names:?}");
        assert!(names[0].ends_with(".log"), "unexpected name: {names:?}");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn writes_the_file_name_and_does_not_split_it() {
        let directory = directory("writes-the-file-name");
        let name = directory.join("application.log");
        let properties = LoggingProperties::from_environment(&environment(&[
            ("NEXT_LOGGING_FILE_NAME", name.to_str().unwrap()),
            ("NEXT_LOGGING_FILE_ROTATION", "never"),
        ]))
        .unwrap();

        let file = properties.file().unwrap();
        let (writer, guard, _dropped) =
            LoggingEventHandler::file_writer(file, &mut Vec::new()).unwrap();
        write_record(writer, guard);

        assert_eq!(file_names(&directory), vec!["application.log".to_owned()]);

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn cleans_the_history_of_a_directory() {
        let directory = directory("cleans-the-history");
        std::fs::write(directory.join("next_20200101.log"), "old").unwrap();
        std::fs::write(directory.join("keep.txt"), "keep").unwrap();

        let properties = LoggingProperties::from_environment(&environment(&[
            ("NEXT_LOGGING_FILE_PATH", directory.to_str().unwrap()),
            ("NEXT_LOGGING_FILE_CLEAN_HISTORY_ON_START", "true"),
        ]))
        .unwrap();

        let file = properties.file().unwrap();
        let (_writer, guard, _dropped) =
            LoggingEventHandler::file_writer(file, &mut Vec::new()).unwrap();
        drop(guard);

        let names = file_names(&directory);
        assert!(
            !names.contains(&"next_20200101.log".to_owned()),
            "the history was not cleaned: {names:?}"
        );
        assert!(names.contains(&"keep.txt".to_owned()));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn reports_the_records_that_were_dropped() {
        assert_eq!(dropped_records_warning(0), None);

        let warning = dropped_records_warning(3).unwrap();

        assert!(warning.contains('3'), "unexpected warning: {warning}");
    }

    #[test]
    fn reports_no_records_when_nothing_was_written() {
        // The handler that never bound its properties holds no count of the
        // records that were dropped, and dropping it reports nothing.
        drop(LoggingEventHandler::default());
    }
}
