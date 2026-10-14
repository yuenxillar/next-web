//! Events published by a retry.

use std::time::Duration;

/// Something a [`Retry`](crate::retry::Retry) did.
///
/// Events are delivered to every subscriber of
/// [`Retry::event_stream`](crate::retry::Retry::event_stream) and are meant for
/// observability. The retried call never waits for a subscriber.
#[derive(Debug, Clone, PartialEq)]
pub enum RetryEvent {
    /// An attempt failed and another one is about to be made.
    Retry {
        /// Number of the attempt that failed, counting from one.
        attempt: u32,
        /// How long the retry waits before the next attempt.
        wait: Duration,
        /// Rendered failure of the attempt.
        message: String,
    },
    /// The call eventually succeeded.
    Success {
        /// Total number of attempts it took.
        attempts: u32,
        /// How long the successful attempt took.
        duration: Duration,
    },
    /// An attempt failed with an error the policy never retries.
    ///
    /// The failure is returned to the caller right away.
    IgnoredError {
        /// Rendered failure of the attempt.
        message: String,
    },
    /// The attempt budget was spent.
    MaxAttemptsReached {
        /// Number of attempts that were made.
        attempts: u32,
        /// Rendered failure of the last attempt, when it produced an error.
        message: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RetryEvent;

    #[test]
    fn events_can_be_cloned_and_compared() {
        let event = RetryEvent::Retry {
            attempt: 2,
            wait: Duration::from_millis(750),
            message: "boom".to_owned(),
        };

        assert_eq!(event.clone(), event);
    }

    #[test]
    fn an_ignored_error_carries_its_message() {
        let event = RetryEvent::IgnoredError {
            message: "rejected".to_owned(),
        };

        match event {
            RetryEvent::IgnoredError { message } => assert_eq!(message, "rejected"),
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
