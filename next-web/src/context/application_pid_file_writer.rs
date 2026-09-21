//! An [`ApplicationEventHandler`] that writes the process id of the application
//! to a file.
//!
//! The handler writes the file once per process, and the name of the file is
//! [`DEFAULT_FILE_NAME`] unless it is configured through the
//! [`PID_FILE_PROPERTY`] or [`PIDFILE_PROPERTY`] properties of the event, or
//! through the [`PIDFILE_ENVIRONMENT_VARIABLE`] environment variable of the
//! process. A failure to write the file is reported as a warning, unless the
//! application asks to fail instead through the
//! [`FAIL_ON_WRITE_ERROR_PROPERTY`] property or the
//! [`FAIL_ON_WRITE_ERROR_ENVIRONMENT_VARIABLE`] environment variable.
//!
//! The file is removed when the handler is dropped, which happens once the
//! application has stopped.
//!
//! # Example
//!
//! ```ignore
//! application.add_event_handlers([Box::new(ApplicationPidFileWriter::default())]);
//! ```

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use next_web_core::{env::ConfigurableEnvironment, Ordered};
use tracing::warn;

use crate::{ApplicationEventHandler, Event};

/// Property that names the file the process id is written to.
pub const PID_FILE_PROPERTY: &str = "next.pid.file";

/// Property that names the file the process id is written to.
pub const PIDFILE_PROPERTY: &str = "next.pidfile";

/// Property that makes a failure to write the file fail the application.
pub const FAIL_ON_WRITE_ERROR_PROPERTY: &str = "next.pid.fail-on-write-error";

/// Environment variable that names the file the process id is written to.
pub const PIDFILE_ENVIRONMENT_VARIABLE: &str = "PIDFILE";

/// Environment variable that makes a failure to write the file fail the
/// application.
pub const FAIL_ON_WRITE_ERROR_ENVIRONMENT_VARIABLE: &str = "PID_FAIL_ON_WRITE_ERROR";

/// The file the process id is written to when none is configured.
pub const DEFAULT_FILE_NAME: &str = "application.pid";

/// The order of the handler, which runs just after the handlers that have the
/// highest precedence.
const ORDER: i32 = i32::MIN + 13;

/// Whether the process id of this process was already written to a file.
static PID_WRITTEN: AtomicBool = AtomicBool::new(false);

/// The event that triggers the writing of the process id file.
///
/// The events that do not carry an environment, such as the event that is
/// published while the application is starting, cannot be used as a trigger,
/// because the name of the file may be configured through the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PidFileTrigger {
    /// The event that is published once the environment is prepared
    EnvironmentPrepared,

    /// The event that is published once the application context is prepared.
    ContextPrepared,

    /// The event that is published once the application context is loaded (the
    /// default).
    #[default]
    ContextLoaded,

    /// The event that is published once the application has started.
    Started,

    /// The event that is published once the application is ready to serve.
    Ready,
}

impl PidFileTrigger {
    /// Returns the trigger of the given event, or `None` when the event does not
    /// carry an environment.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to inspect.
    pub fn of(event: &Event<'_>) -> Option<Self> {
        match event {
            Event::EnvironmentPrepared(_) => Some(Self::EnvironmentPrepared),
            Event::ContextPrepared { .. } => Some(Self::ContextPrepared),
            Event::ContextLoaded { .. } => Some(Self::ContextLoaded),
            Event::Started { .. } => Some(Self::Started),
            Event::Ready { .. } => Some(Self::Ready),
            Event::Starting { .. } | Event::Error { .. } => None,
        }
    }

    /// Returns whether the given event triggers the writing of the process id
    /// file.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to inspect.
    pub fn matches(&self, event: &Event<'_>) -> bool {
        Self::of(event) == Some(*self)
    }
}

/// An [`ApplicationEventHandler`] that writes the process id of the application
/// to a file.
///
/// The handler is not registered by default: add it to an application that has
/// to expose its process id, for example as
/// `application.add_event_handlers([Box::new(ApplicationPidFileWriter::default())])`.
pub struct ApplicationPidFileWriter {
    order: i32,
    file: PathBuf,
    trigger: PidFileTrigger,
    written_file: Option<PathBuf>,
}

impl ApplicationPidFileWriter {
    /// Creates a handler that writes the process id to the given file.
    ///
    /// The name of the file may still be overridden by the properties and the
    /// environment variable that configure it.
    ///
    /// # Arguments
    ///
    /// * `file` - The file to write the process id to.
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self {
            order: ORDER,
            file: file.into(),
            trigger: PidFileTrigger::default(),
            written_file: None,
        }
    }

    /// Sets the order of the handler.
    ///
    /// # Arguments
    ///
    /// * `order` - The order to use.
    pub fn set_order(&mut self, order: i32) {
        self.order = order;
    }

    /// Sets the event that triggers the writing of the process id file.
    ///
    /// # Arguments
    ///
    /// * `trigger` - The trigger to use.
    pub fn set_trigger(&mut self, trigger: PidFileTrigger) {
        self.trigger = trigger;
    }

    /// Returns the file the process id is written to when none is configured.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the event that triggers the writing of the process id file.
    pub fn trigger(&self) -> PidFileTrigger {
        self.trigger
    }

    /// Returns the file the process id is written to for the given event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event that carries the environment of the application.
    fn pid_file(&self, event: &Event<'_>) -> PathBuf {
        self.property(event, &[PID_FILE_PROPERTY, PIDFILE_PROPERTY])
            .or_else(|| environment_variable(PIDFILE_ENVIRONMENT_VARIABLE))
            .map(PathBuf::from)
            .unwrap_or_else(|| self.file.clone())
    }

    /// Returns whether a failure to write the process id file fails the
    /// application.
    ///
    /// # Arguments
    ///
    /// * `event` - The event that carries the environment of the application.
    fn fail_on_write_error(&self, event: &Event<'_>) -> bool {
        self.property(event, &[FAIL_ON_WRITE_ERROR_PROPERTY])
            .or_else(|| environment_variable(FAIL_ON_WRITE_ERROR_ENVIRONMENT_VARIABLE))
            .map(|value| parse_bool(&value).unwrap_or(false))
            .unwrap_or(false)
    }

    /// Returns the value of the first of the given properties that the
    /// environment of the event has.
    ///
    /// # Arguments
    ///
    /// * `event` - The event that carries the environment of the application.
    /// * `names` - The names of the properties to look up.
    fn property(&self, event: &Event<'_>, names: &[&str]) -> Option<String> {
        let environment = environment(event)?;
        names.iter().find_map(|name| environment.get_property(name))
    }

    /// Writes the process id of this process to the given file.
    ///
    /// # Arguments
    ///
    /// * `file` - The file to write the process id to.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] when the file cannot be written.
    fn write_pid_file(&mut self, file: &Path) -> std::io::Result<()> {
        fs::write(file, std::process::id().to_string())?;
        self.written_file = Some(file.to_owned());
        Ok(())
    }
}

impl Default for ApplicationPidFileWriter {
    fn default() -> Self {
        Self::new(DEFAULT_FILE_NAME)
    }
}

impl Ordered for ApplicationPidFileWriter {
    fn order(&self) -> i32 {
        self.order
    }
}

impl ApplicationEventHandler for ApplicationPidFileWriter {
    /// Writes the process id of the application to its file.
    ///
    /// The file is written the first time the trigger event of this handler is
    /// received, and only once per process.
    ///
    /// # Arguments
    ///
    /// * `event` - The event that was received.
    ///
    /// # Panics
    ///
    /// Panics when the file cannot be written and the application asks to fail
    /// on a write error.
    fn handle_event(&mut self, event: Event) {
        if !self.trigger.matches(&event) {
            return;
        }
        if PID_WRITTEN.swap(true, Ordering::SeqCst) {
            return;
        }

        let file = self.pid_file(&event);
        if let Err(error) = self.write_pid_file(&file) {
            let message = format!("Cannot create pid file {}", file.display());
            if self.fail_on_write_error(&event) {
                panic!("{message}: {error}");
            }
            warn!("{message}: {error}");
        }
    }
}

impl Drop for ApplicationPidFileWriter {
    /// Removes the file that was written, once the application has stopped.
    fn drop(&mut self) {
        let Some(file) = self.written_file.take() else {
            return;
        };

        if let Err(error) = fs::remove_file(&file) {
            // Something else removed the file, which is not a problem.
            if error.kind() != std::io::ErrorKind::NotFound {
                warn!("Cannot remove pid file {}: {error}", file.display());
            }
        }
    }
}

/// Returns the environment of the given event, when the event carries one.
///
/// # Arguments
///
/// * `event` - The event to inspect.
fn environment<'a>(event: &'a Event<'_>) -> Option<&'a dyn ConfigurableEnvironment> {
    match event {
        Event::EnvironmentPrepared(payload) => Some(&*payload.environment),
        Event::ContextPrepared { context, .. }
        | Event::ContextLoaded { context, .. }
        | Event::Started { context, .. }
        | Event::Ready { context, .. } => Some(context.environment()),
        Event::Starting { .. } | Event::Error { .. } => None,
    }
}

/// Returns the value of the given environment variable of the process, if it is
/// set and upper case, or if it is set in lower case.
///
/// # Arguments
///
/// * `name` - The name of the environment variable, in upper case.
fn environment_variable(name: &str) -> Option<String> {
    std::env::var(name)
        .or_else(|_| std::env::var(name.to_lowercase()))
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Parses a boolean value, accepting `true` and `false` in any case.
///
/// # Arguments
///
/// * `value` - The value to parse.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Allows the process id to be written again, which is only needed by tests.
#[cfg(test)]
fn reset() {
    PID_WRITTEN.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::Mutex,
        time::{SystemTime, UNIX_EPOCH},
    };

    use next_web_core::env::StandardEnvironment;
    use next_web_core::util::indexmap::IndexMap;

    use crate::context::DefaultApplicationContext;
    use crate::env::MapPropertySource;
    use crate::EnvironmentPreparedPayload;

    use super::*;

    /// Guards the tests that write a process id, because only one process id is
    /// written per process.
    static TEST_GUARD: Mutex<()> = Mutex::new(());

    /// Locks the guard of the tests that write a process id.
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reset();
        guard
    }

    /// Returns a file in the temporary directory that the test owns.
    fn temp_file(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "next-web-pid-file-{name}-{}-{unique}.pid",
            std::process::id()
        ))
    }

    /// Returns an environment holding the given properties.
    fn environment(properties: &[(&str, &str)]) -> StandardEnvironment {
        let mut environment = StandardEnvironment::new();
        let properties: IndexMap<String, String> = properties
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        environment
            .property_sources()
            .add_first(Box::new(MapPropertySource::new(
                "test".to_owned(),
                properties,
            )));
        environment
    }

    /// Sends the given event to the given handler.
    fn send(handler: &mut ApplicationPidFileWriter, event: Event<'_>) {
        handler.handle_event(event);
    }

    /// Sends an environment prepared event with the given environment.
    fn send_environment_prepared(
        handler: &mut ApplicationPidFileWriter,
        environment: &mut StandardEnvironment,
    ) {
        send(
            handler,
            Event::EnvironmentPrepared(EnvironmentPreparedPayload {
                args: &[],
                environment,
                resource_loader: None,
                additional_profiles: Vec::new(),
            }),
        );
    }

    /// Returns the process id that is stored in the given file.
    fn read_pid(file: &Path) -> String {
        fs::read_to_string(file).expect("the process id file should be readable")
    }

    #[test]
    fn uses_the_default_file_and_the_default_trigger() {
        let mut writer = ApplicationPidFileWriter::default();

        assert_eq!(writer.file(), Path::new(DEFAULT_FILE_NAME));
        assert_eq!(writer.trigger(), PidFileTrigger::EnvironmentPrepared);
        assert_eq!(writer.order(), i32::MIN + 13);

        writer.set_order(1_000);

        assert_eq!(writer.order(), 1_000);
    }

    #[test]
    fn writes_the_process_id_of_the_process() {
        let _guard = guard();
        let file = temp_file("process-id");
        let mut writer = ApplicationPidFileWriter::new(&file);
        let mut environment = environment(&[]);

        send_environment_prepared(&mut writer, &mut environment);

        assert_eq!(read_pid(&file), std::process::id().to_string());
    }

    #[test]
    fn does_not_write_the_process_id_for_another_trigger() {
        let _guard = guard();
        let file = temp_file("other-trigger");
        let mut writer = ApplicationPidFileWriter::new(&file);
        writer.set_trigger(PidFileTrigger::Ready);
        let mut environment = environment(&[]);

        send_environment_prepared(&mut writer, &mut environment);

        assert!(!file.exists());
    }

    #[test]
    fn writes_the_process_id_once_per_process() {
        let _guard = guard();
        let file = temp_file("once");
        let other = temp_file("once-other");
        let mut environment = environment(&[]);

        let mut writer = ApplicationPidFileWriter::new(&file);
        send_environment_prepared(&mut writer, &mut environment);
        assert!(file.exists());

        let mut other_writer = ApplicationPidFileWriter::new(&other);
        send_environment_prepared(&mut other_writer, &mut environment);
        assert!(!other.exists());
    }

    #[test]
    fn takes_the_file_from_the_properties() {
        let _guard = guard();
        let file = temp_file("configured");
        let mut writer = ApplicationPidFileWriter::new(temp_file("ignored"));
        let mut environment = environment(&[(PID_FILE_PROPERTY, &file.to_string_lossy())]);

        send_environment_prepared(&mut writer, &mut environment);

        assert_eq!(read_pid(&file), std::process::id().to_string());
        assert!(!writer.file().exists());
    }

    #[test]
    fn takes_the_file_from_the_legacy_property() {
        let _guard = guard();
        let file = temp_file("legacy");
        let mut writer = ApplicationPidFileWriter::new(temp_file("ignored"));
        let mut environment = environment(&[(PIDFILE_PROPERTY, &file.to_string_lossy())]);

        send_environment_prepared(&mut writer, &mut environment);

        assert!(file.exists());
    }

    #[test]
    fn ignores_a_file_that_cannot_be_created() {
        let _guard = guard();
        let mut writer =
            ApplicationPidFileWriter::new(std::env::temp_dir().join("next-web/missing/dir"));
        let mut environment = environment(&[]);

        send_environment_prepared(&mut writer, &mut environment);
    }

    #[test]
    #[should_panic(expected = "Cannot create pid file")]
    fn fails_on_a_file_that_cannot_be_created_when_asked_to() {
        let _guard = guard();
        let mut writer =
            ApplicationPidFileWriter::new(std::env::temp_dir().join("next-web/missing/dir"));
        let mut environment = environment(&[(FAIL_ON_WRITE_ERROR_PROPERTY, "true")]);

        send_environment_prepared(&mut writer, &mut environment);
    }

    #[test]
    fn removes_the_file_when_the_application_stops() {
        let _guard = guard();
        let file = temp_file("removed");
        let mut environment = environment(&[]);

        let mut writer = ApplicationPidFileWriter::new(&file);
        send_environment_prepared(&mut writer, &mut environment);
        assert!(file.exists());

        drop(writer);

        assert!(!file.exists());
    }

    #[test]
    fn reports_the_trigger_of_every_event() {
        assert_eq!(PidFileTrigger::of(&Event::Starting { args: &[] }), None);
        assert_eq!(
            PidFileTrigger::of(&Event::EnvironmentPrepared(EnvironmentPreparedPayload {
                args: &[],
                environment: &mut StandardEnvironment::new(),
                resource_loader: None,
                additional_profiles: Vec::new(),
            })),
            Some(PidFileTrigger::EnvironmentPrepared)
        );

        let mut context = DefaultApplicationContext::default();
        assert_eq!(
            PidFileTrigger::of(&Event::ContextPrepared {
                args: &[],
                context: &mut context,
            }),
            Some(PidFileTrigger::ContextPrepared)
        );

        let mut context = DefaultApplicationContext::default();
        assert_eq!(
            PidFileTrigger::of(&Event::ContextLoaded {
                args: &[],
                context: &mut context,
            }),
            Some(PidFileTrigger::ContextLoaded)
        );

        let mut context = DefaultApplicationContext::default();
        assert_eq!(
            PidFileTrigger::of(&Event::Started {
                args: &[],
                context: &mut context,
                time_taken: None,
            }),
            Some(PidFileTrigger::Started)
        );

        let mut context = DefaultApplicationContext::default();
        assert_eq!(
            PidFileTrigger::of(&Event::Ready {
                args: &[],
                context: &mut context,
                time_taken: None,
            }),
            Some(PidFileTrigger::Ready)
        );

        let error = std::io::Error::other("boom");
        assert_eq!(
            PidFileTrigger::of(&Event::Error {
                args: &[],
                err: &error
            }),
            None
        );
    }

    #[test]
    fn matches_the_trigger_of_an_event() {
        let mut environment = StandardEnvironment::new();

        assert!(
            PidFileTrigger::EnvironmentPrepared.matches(&Event::EnvironmentPrepared(
                EnvironmentPreparedPayload {
                    args: &[],
                    environment: &mut environment,
                    resource_loader: None,
                    additional_profiles: Vec::new(),
                }
            ))
        );
        assert!(!PidFileTrigger::Ready.matches(&Event::ContextLoaded {
            args: &[],
            context: &mut DefaultApplicationContext::default(),
        }));
    }

    #[test]
    fn parses_boolean_values() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool(" TRUE "), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("yes"), None);
    }

    #[test]
    fn reads_the_environment_variables_of_the_process() {
        // A variable that every process has, to check how the name of a
        // variable is looked up without changing the environment of the test.
        let path = environment_variable("PATH");
        let lower_case = environment_variable("path");

        assert_eq!(path.is_some(), lower_case.is_some());
        assert!(environment_variable("NEXT_WEB_VARIABLE_THAT_IS_NOT_SET").is_none());
    }
}
