use chrono::Local;
use next_web_core::env::Environment;
use tracing::info;

use crate::{next_web_application::StandardStartup, NextWebVersion};

/// Logs application information on startup.
pub struct StartupInfoLogger<'a> {
    source_name: Option<&'static str>,
    environment: &'a dyn Environment,
}

impl<'a> StartupInfoLogger<'a> {
    /// Creates a new [`StartupInfoLogger`].
    pub fn new(source_name: Option<&'static str>, environment: &'a dyn Environment) -> Self {
        Self {
            source_name,
            environment,
        }
    }

    /// Logs the "starting" message at info level and the "running" message
    /// at debug level.
    pub fn log_starting(&self) {
        info!("{}", self.get_starting_message());
        info!("{}", self.get_running_message());
        self.print_runtime_message();
    }

    /// Logs the "started" message.
    pub fn log_started(&self, startup: &StandardStartup) {
        println!("\n{}", self.get_started_message(startup));
    }

    /// Builds the "Starting ..." line.
    fn get_starting_message(&self) -> String {
        let mut msg = String::new();
        msg.push_str("Starting");
        msg.push(' ');
        msg.push_str(
            self.source_name
                .and_then(|s| s.rsplit("::").next())
                .unwrap_or_else(|| "NextWebApplication")
                .to_owned()
                .as_mut_str(),
        );
        self.append_context(&mut msg);
        msg
    }

    /// Builds the debug line "Running with ... vX.Y.Z, ...".
    fn get_running_message(&self) -> String {
        let mut msg = String::new();
        msg.push_str("Running with Next Web ");
        self.append_application_version(&mut msg);
        msg
    }

    /// Prints the runtime message to the log.
    fn print_runtime_message(&self) {
        info!("Starting Async Runtime: [Tokio/1.44.1]");
        info!("Starting HTTP  Server:  [Axum/0.8.4]");
    }

    /// Builds the "Started ... in N seconds (process running for M)" line.
    fn get_started_message(&self, startup: &StandardStartup) -> String {
        let mut msg = String::new();
        self.append_application_name(&mut msg);
        msg.push('\n');
        self.append_listening_on(&mut msg);
        msg.push('\n');
        self.append_started_at(&mut msg);
        msg.push('\n');
        self.append_startup_time(&mut msg, startup);
        msg.push('\n');
        self.append_pid(&mut msg);
        msg.push('\n');

        msg
    }

    /// Appends the listening address and port of the application.
    fn append_listening_on(&self, msg: &mut String) {
        self.append(
            msg,
            "Application Listening on:  ",
            || {
                format!(
                    "{}:{}",
                    self.environment
                        .get_property_or_default("next.server.address", "0.0.0.0"),
                    self.environment
                        .get_property_or_default("next.server.port", "11000")
                )
            },
            false,
        );
    }

    /// Appends the started at timestamp of the application.
    fn append_started_at(&self, msg: &mut String) {
        self.append(
            msg,
            "Application Started   at:  ",
            || Local::now().to_string(),
            false,
        );
    }

    /// Appends the startup time of the application.
    fn append_startup_time(&self, msg: &mut String, startup: &StandardStartup) {
        // msg.push_str(startup.action());
        // msg.push_str(" in ");
        // msg.push_str(&format!(
        //     "{:.3}",
        //     startup
        //         .time_taken_to_started()
        //         .ok()
        //         .map(|d| d.as_secs_f64())
        //         .unwrap_or(0.0)
        // ));
        // msg.push_str(" seconds");
        // if let Some(uptime) = startup.process_uptime() {
        //     msg.push_str(&format!(
        //         " (process running for {:.3})",
        //         uptime.as_secs_f64()
        //     ));
        // }

        self.append(
            msg,
            "Application Startup time:  ",
            || {
                startup
                    .time_taken_to_started()
                    .ok()
                    .map(|dur| format!("{dur:?}"))
                    .unwrap_or("0.01s".to_owned())
            },
            false,
        );
    }

    /// Appends the application name, falling back to `"application"`.
    fn append_application_name(&self, msg: &mut String) {
        self.append(
            msg,
            "Application Name      is:  ",
            || {
                self.source_name
                    .and_then(|s| s.rsplit("::").next())
                    .unwrap_or_else(|| "NextWebApplication")
                    .to_owned()
            },
            false,
        );
    }

    /// Appends a version with a `v` prefix.
    #[allow(dead_code)]
    fn append_version(&self, msg: &mut String, version: Option<&str>) {
        self.append(
            msg,
            "v",
            || version.map(String::from).unwrap_or("latest".to_owned()),
            true,
        );
    }

    /// Appends the application version.
    fn append_application_version(&self, msg: &mut String) {
        self.append(
            msg,
            "v",
            || {
                self.environment
                    .get_property_or_default(
                        "next.application.version",
                        NextWebVersion::get_version(),
                    )
                    .to_owned()
            },
            true,
        );
    }

    /// Appends the process ID.
    fn append_pid(&self, msg: &mut String) {
        self.append(
            msg,
            "Application Process   ID:  ",
            || std::process::id().to_string(),
            false,
        );
    }

    /// Appends the startup context: executable path, user, and working directory.
    fn append_context(&self, msg: &mut String) {
        let mut context = String::new();

        // Current executable path, mirroring `ApplicationHome.getSource()`.
        if let Ok(exe) = std::env::current_exe() {
            context.push_str(&exe.to_string_lossy());
        }

        // Startup user.
        self.append_to(&mut context, "started by ", user_name);

        // Working directory.
        self.append_to(&mut context, "in ", || {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or("Unknown".to_owned())
        });

        if !context.is_empty() {
            msg.push_str(" (");
            msg.push_str(&context);
            msg.push(')');
        }
    }

    /// Appends `prefix + value` to `message`, skipping empty values.
    fn append<F>(&self, message: &mut String, prefix: &str, call: F, whitespace: bool)
    where
        F: FnOnce() -> String,
    {
        self.append_with_default(message, prefix, call, "unknown", whitespace)
    }

    /// Appends `prefix + value` with a fallback default when the value is
    /// absent or empty.
    fn append_with_default<F>(
        &self,
        message: &mut String,
        prefix: &str,
        call: F,
        default_value: &str,
        whitespace: bool,
    ) where
        F: FnOnce() -> String,
    {
        let owned = call();
        let value: &str = if owned.is_empty() {
            default_value
        } else {
            &owned
        };
        if !value.is_empty() {
            if !message.is_empty() && whitespace {
                message.push(' ');
            }
            message.push_str(prefix);
            message.push_str(value);
        }
    }

    /// Same as [`append`](Self::append) but writes into a separate builder,
    fn append_to<F>(&self, message: &mut String, prefix: &str, call: F)
    where
        F: FnOnce() -> String,
    {
        self.append_with_default(message, prefix, call, "", true);
    }
}

/// Returns the current user name, mirroring `user.name`.
fn user_name() -> String {
    let key = if cfg!(windows) { "USERNAME" } else { "USER" };
    std::env::var(key).unwrap_or("Unknown".to_owned())
}
