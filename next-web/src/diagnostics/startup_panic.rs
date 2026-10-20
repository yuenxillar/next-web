//! The panic of a startup, reported once by the framework.
//!
//! A panic that stops an application from starting is reported twice by default:
//! the hook of the standard library prints the message and the location of the
//! panic, and the framework prints the analysis of the failure afterwards, with a
//! description that holds the message but not the location — which is the part an
//! application needs in order to find the panic.
//!
//! The framework therefore installs a hook of its own while an application starts.
//! The hook keeps the message and the location of the panic, and it stays quiet
//! while the startup is running, so the report of the framework is the only output
//! of a panic that stops a startup. The hook is not installed for the rest of the
//! life of the application, and a panic that happens when no startup is running
//! reaches the hook of the standard library unchanged.
//!
//! # What a report holds
//!
//! The description of the failure is the message of the panic, followed by the
//! location of it:
//!
//! ```text
//! Description:
//!
//! is panic1
//!
//!     at examples/application.rs:20:9
//! ```

use std::{
    any::Any,
    fmt,
    panic::{self, PanicHookInfo},
    sync::{
        Arc, Mutex, Once, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

/// The hook the framework installed before its own, which reports the panics of
/// an application that is not starting.
static PREVIOUS_HOOK: OnceLock<Arc<dyn Fn(&PanicHookInfo<'_>) + Send + Sync>> = OnceLock::new();

/// Whether an application is starting on the process right now.
///
/// The flag is what tells the hook whether the framework is about to report the
/// panic itself: a panic of a startup is reported by the framework, and a panic of
/// anything else reaches the hook of the standard library.
static STARTING: AtomicBool = AtomicBool::new(false);

/// The panic a startup stopped on, when one did.
static RECORDED: Mutex<Option<StartupPanic>> = Mutex::new(None);

/// The panic an application stopped on while it started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupPanic {
    message: String,
    location: Option<String>,
}

impl StartupPanic {
    /// Creates the panic an application stopped on.
    ///
    /// # Arguments
    ///
    /// * `message` - The message of the panic.
    /// * `location` - The file, the line and the column of the panic, when the
    ///   panic carries them.
    pub fn new(message: impl Into<String>, location: Option<String>) -> Self {
        Self {
            message: message.into(),
            location,
        }
    }

    /// Returns the message of the panic.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the location of the panic, which is the file, the line and the
    /// column it was raised at.
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    /// Returns the description of the failure the panic caused.
    ///
    /// The description is the message of the panic, followed by the location of
    /// it when the panic carries one, so what the framework reports names the file
    /// and the line an application has to look at.
    pub fn description(&self) -> String {
        match self.location() {
            Some(location) => format!("{}\n\n    at {location}", self.message),
            None => self.message.clone(),
        }
    }
}

impl fmt::Display for StartupPanic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.description())
    }
}

/// Keeps the hook of the framework quiet while the startup that holds it runs.
///
/// The guard is held by the future that starts an application, so the hook stays
/// quiet for the whole startup and reports the panics of the application once the
/// startup is over.
#[derive(Debug)]
pub struct StartupPanicGuard {
    /// Whether the guard still holds the flag.
    held: bool,
}

impl StartupPanicGuard {
    /// Marks the start of a startup, so that the hook of the framework reports
    /// the panics of it.
    pub fn new() -> Self {
        STARTING.store(true, Ordering::SeqCst);

        Self { held: true }
    }

    /// Releases the flag, which hands the panics of the process back to the hook
    /// of the standard library.
    pub fn release(&mut self) {
        if self.held {
            self.held = false;
            STARTING.store(false, Ordering::SeqCst);
        }
    }
}

impl Default for StartupPanicGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for StartupPanicGuard {
    fn drop(&mut self) {
        self.release();
    }
}

/// Installs the hook that reports the panic of a startup once.
///
/// Installing it twice keeps the hook that is already installed, so every
/// application of a process reports a panic the same way.
pub fn install_hook() {
    static INSTALL: Once = Once::new();

    INSTALL.call_once(|| {
        let previous = panic::take_hook();
        let _ = PREVIOUS_HOOK.set(Arc::new(previous));

        panic::set_hook(Box::new(|info| {
            if STARTING.load(Ordering::SeqCst) {
                record(info);

                return;
            }

            if let Some(previous) = PREVIOUS_HOOK.get() {
                previous(info);
            }
        }));
    });
}

/// Keeps the panic of the application that is starting, when one is.
///
/// The panic is kept once: the first panic of a startup is the one the
/// application stopped on, and a startup that reports it takes it out.
///
/// # Arguments
///
/// * `info` - The panic the hook was called with.
fn record(info: &PanicHookInfo<'_>) {
    let mut recorded = RECORDED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if recorded.is_some() {
        return;
    }

    *recorded = Some(StartupPanic::new(
        payload_message(info.payload()),
        info.location()
            .map(|location| format!("{}:{}:{}", location.file(), location.line(), location.column())),
    ));
}

/// Records a panic without the hook, which is what the tests of the module do.
///
/// # Arguments
///
/// * `panic` - The panic to record.
#[cfg(test)]
fn record_without_hook(panic: StartupPanic) {
    let mut recorded = RECORDED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *recorded = Some(panic);
}

/// Takes the panic the startup stopped on, if one was recorded.
///
/// This is what the framework reports, so the panic is reported once: a later
/// call answers with `None` until another startup records one.
pub fn take_recorded() -> Option<StartupPanic> {
    RECORDED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
}

/// Returns whether a startup is running on the process, which is where a panic is
/// reported by the framework instead of by the hook of the standard library.
pub fn is_starting() -> bool {
    STARTING.load(Ordering::SeqCst)
}

/// Returns the message of a panic payload.
///
/// # Arguments
///
/// * `payload` - The payload the panic was raised with.
pub fn payload_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| String::from("Unknown panic"))
}

#[cfg(test)]
mod tests {
    use super::{StartupPanic, StartupPanicGuard, is_starting, payload_message, take_recorded};

    #[test]
    fn a_description_names_the_location_of_the_panic() {
        let panic = StartupPanic::new("is panic1", Some(String::from("examples/application.rs:20:9")));

        assert_eq!(
            panic.description(),
            "is panic1\n\n    at examples/application.rs:20:9"
        );
        assert_eq!(panic.message(), "is panic1");
        assert_eq!(panic.location(), Some("examples/application.rs:20:9"));
    }

    #[test]
    fn a_description_of_a_panic_without_a_location_is_the_message() {
        let panic = StartupPanic::new("is panic1", None);

        assert_eq!(panic.description(), "is panic1");
        assert_eq!(panic.to_string(), "is panic1");
    }

    #[test]
    fn a_recorded_panic_is_taken_once() {
        // The test records a panic the way the hook does, so that taking it does
        // not depend on a panic of the test itself.
        super::record_without_hook(StartupPanic::new("is panic1", None));

        assert!(take_recorded().is_some());
        assert!(take_recorded().is_none());
    }

    #[test]
    fn the_hook_records_the_message_and_the_location_of_a_panic() {
        super::install_hook();
        let mut guard = StartupPanicGuard::new();

        let _ = std::panic::catch_unwind(|| panic!("is panic1"));
        guard.release();

        let panic = take_recorded().expect("the hook recorded the panic");
        assert_eq!(panic.message(), "is panic1");
        assert!(
            panic.location().is_some(),
            "the hook keeps the location of the panic"
        );
    }

    #[test]
    fn the_flag_of_the_startup_is_held_by_the_guard_it_returns() {
        let mut guard = StartupPanicGuard::new();
        assert!(is_starting());

        guard.release();
        assert!(!is_starting());

        // Dropping a released guard leaves the flag alone.
        drop(guard);
        assert!(!is_starting());
    }

    #[test]
    fn the_message_of_a_panic_of_a_str_or_a_string_is_read() {
        assert_eq!(payload_message(&"is panic1"), "is panic1");
        assert_eq!(payload_message(&String::from("is panic2")), "is panic2");
    }
}
