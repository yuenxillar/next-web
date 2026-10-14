//! Events published by a time limiter.

use std::time::Duration;

/// Something a time limiter did.
///
/// Events are delivered to every subscriber of
/// [`TimeLimiter::event_stream`](crate::timelimiter::TimeLimiter::event_stream)
/// and are meant for observability: metrics exporters, dashboards and logs. The
/// guarded call never waits for a subscriber, and an event is dropped when
/// nobody is listening.
#[derive(Debug, Clone, PartialEq)]
pub enum TimeLimiterEvent {
    /// A guarded call finished within the time limit.
    Success {
        /// How long the call took.
        duration: Duration,
    },
    /// A guarded call failed before the time limit was reached.
    Error {
        /// How long the call took before failing.
        duration: Duration,
        /// Rendered failure of the call.
        message: String,
    },
    /// A guarded call outlived the time limit.
    Timeout {
        /// The limit the call exceeded.
        timeout: Duration,
    },
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::TimeLimiterEvent;

    #[test]
    fn success_and_error_events_keep_their_duration() {
        let success = TimeLimiterEvent::Success {
            duration: Duration::from_millis(5),
        };
        let error = TimeLimiterEvent::Error {
            duration: Duration::from_millis(7),
            message: "boom".to_owned(),
        };

        assert_eq!(success.clone(), success);
        assert_eq!(error.clone(), error);
        assert_ne!(success, error);
    }

    #[test]
    fn a_timeout_event_carries_the_limit_that_was_exceeded() {
        let event = TimeLimiterEvent::Timeout {
            timeout: Duration::from_secs(1),
        };

        match event {
            TimeLimiterEvent::Timeout { timeout } => {
                assert_eq!(timeout, Duration::from_secs(1));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
