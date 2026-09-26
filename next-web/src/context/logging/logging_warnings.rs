//! The reports of the failures of the logging functionality.
//!
//! The writer of the log files runs on the thread the records are written from,
//! which the logging functionality starts itself. A failure of the writer
//! cannot be reported through the logging functionality from that thread: the
//! records of the report are added to the channel the writer reads from, so a
//! full channel blocks the writer, which is the very thread that would have to
//! empty it.
//!
//! A [`WarningReporter`] hands the warnings of the writer to a thread of its
//! own, which logs them, so the failures of the writer reach the console and
//! the log files without the writer waiting for anything.

use std::sync::mpsc::{self, Receiver, SyncSender};

/// The number of the warnings that wait for the thread that logs them.
///
/// A warning is dropped rather than waited for when the thread cannot keep up,
/// because the thread that writes the records is never allowed to be blocked.
const WARNING_CAPACITY: usize = 64;

/// The name of the thread that logs the reported warnings.
const WARNING_THREAD_NAME: &str = "next-web-logging-warnings";

/// Reports the warnings of the logging functionality.
///
/// The warnings are handed to the thread of the reporter, which logs them with
/// the level `warn`. A warning that the thread cannot keep up with is dropped,
/// so reporting a warning never blocks the thread that reports it.
#[derive(Debug, Clone)]
pub struct WarningReporter {
    /// The warnings that wait for the thread that logs them, which is empty
    /// when the warnings are dropped.
    warnings: Option<SyncSender<String>>,
}

impl WarningReporter {
    /// Creates a reporter that logs the warnings it is given.
    ///
    /// The warnings are logged by a thread of the reporter, which ends once
    /// every reporter of the channel has been dropped. The warnings are dropped
    /// when the thread cannot be started, so a failure to log a warning never
    /// stops the application from starting.
    pub fn logging() -> Self {
        let (reporter, warnings) = Self::channel();

        let started = std::thread::Builder::new()
            .name(WARNING_THREAD_NAME.to_owned())
            .spawn(move || {
                for warning in warnings {
                    tracing::warn!(target: "next_web::logging", "{warning}");
                }
            });

        match started {
            Ok(_thread) => reporter,
            Err(_) => Self::none(),
        }
    }

    /// Creates a reporter that drops the warnings it is given.
    pub fn none() -> Self {
        Self { warnings: None }
    }

    /// Creates a reporter and the receiver of the warnings it is given, which
    /// the caller logs.
    pub fn channel() -> (Self, Receiver<String>) {
        let (warnings, receiver) = mpsc::sync_channel(WARNING_CAPACITY);

        (
            Self {
                warnings: Some(warnings),
            },
            receiver,
        )
    }

    /// Reports the given warning.
    ///
    /// The warning is dropped when the thread that logs the warnings cannot
    /// keep up, so the thread that reports it is never blocked.
    ///
    /// # Arguments
    ///
    /// * `warning` - The warning to report.
    pub fn report(&self, warning: impl Into<String>) {
        let Some(warnings) = self.warnings.as_ref() else {
            return;
        };

        let _ = warnings.try_send(warning.into());
    }
}

impl Default for WarningReporter {
    /// Creates a reporter that drops the warnings it is given.
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_warnings_of_the_thread_that_logs_them() {
        let (reporter, warnings) = WarningReporter::channel();

        reporter.report("the first warning");
        reporter.report("the second warning");

        assert_eq!(warnings.try_recv().as_deref(), Ok("the first warning"));
        assert_eq!(warnings.try_recv().as_deref(), Ok("the second warning"));
        assert!(warnings.try_recv().is_err());
    }

    #[test]
    fn ends_the_channel_of_the_warnings_when_the_reporters_are_dropped() {
        let (reporter, warnings) = WarningReporter::channel();

        drop(reporter);

        assert!(warnings.recv().is_err());
    }

    #[test]
    fn drops_the_warnings_that_no_thread_logs() {
        let (reporter, warnings) = WarningReporter::channel();

        drop(warnings);

        // The warning of a channel that no thread reads is dropped rather than
        // reported, which keeps the thread that reports it running.
        reporter.report("a warning of a closed channel");
    }

    #[test]
    fn drops_the_warnings_of_a_reporter_that_holds_no_thread() {
        WarningReporter::none().report("a warning of a reporter without a thread");
    }
}
