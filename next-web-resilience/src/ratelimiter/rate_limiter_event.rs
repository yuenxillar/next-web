//! Events published by a rate limiter.

use std::time::Duration;

/// Something a rate limiter did.
///
/// Events are delivered to every subscriber of
/// [`RateLimiter::event_stream`](crate::ratelimiter::RateLimiter::event_stream)
/// and are meant for observability: metrics exporters, dashboards and logs. The
/// guarded call never waits for a subscriber.
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimiterEvent {
    /// A permission was granted.
    PermissionAcquired {
        /// How long the acquiring call took.
        ///
        /// This is the time between the start of the call and the grant, which
        /// is the time actually slept when the limiter had to wait for the next
        /// period. It is [`Duration::ZERO`] for
        /// [`acquire_permission`](crate::ratelimiter::RateLimiter::acquire_permission),
        /// which never waits.
        waited: Duration,
    },
    /// A permission was refused, so the guarded call was not executed.
    ///
    /// Waiting until the next period was either not attempted or did not fit in
    /// the timeout the caller gave.
    PermissionRejected,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RateLimiterEvent;

    #[test]
    fn events_can_be_cloned_and_compared() {
        let event = RateLimiterEvent::PermissionAcquired {
            waited: Duration::from_millis(250),
        };

        assert_eq!(event.clone(), event);
    }

    #[test]
    fn different_events_are_not_equal() {
        assert_ne!(
            RateLimiterEvent::PermissionRejected,
            RateLimiterEvent::PermissionAcquired {
                waited: Duration::ZERO
            }
        );
    }

    #[test]
    fn an_acquired_event_keeps_the_waiting_time() {
        let event = RateLimiterEvent::PermissionAcquired {
            waited: Duration::from_millis(120),
        };

        match event {
            RateLimiterEvent::PermissionAcquired { waited } => {
                assert_eq!(waited, Duration::from_millis(120));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
