//! Events published by a bulkhead.

use std::time::Duration;

/// Something a bulkhead did.
///
/// Events are delivered to every subscriber of
/// [`Bulkhead::event_stream`](crate::bulkhead::Bulkhead::event_stream) and are
/// meant for observability: metrics exporters, dashboards and logs. The guarded
/// call never waits for a subscriber.
#[derive(Debug, Clone, PartialEq)]
pub enum BulkheadEvent {
    /// A call was admitted and now occupies one of the bulkhead's slots.
    CallPermitted {
        /// Slots left after this call took its permit.
        available_concurrent_calls: u32,
    },
    /// A call was rejected because no slot was free in time.
    ///
    /// The guarded call was never executed.
    CallRejected {
        /// Slots left when the call was rejected; normally zero.
        available_concurrent_calls: u32,
    },
    /// A call released its slot.
    CallFinished {
        /// How long the call held its permit.
        duration: Duration,
    },
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::BulkheadEvent;

    #[test]
    fn events_can_be_cloned_and_compared() {
        let event = BulkheadEvent::CallPermitted {
            available_concurrent_calls: 4,
        };

        assert_eq!(event.clone(), event);
        assert_ne!(
            event,
            BulkheadEvent::CallRejected {
                available_concurrent_calls: 4
            }
        );
    }

    #[test]
    fn a_finished_event_keeps_its_duration() {
        let event = BulkheadEvent::CallFinished {
            duration: Duration::from_millis(250),
        };

        match event {
            BulkheadEvent::CallFinished { duration } => {
                assert_eq!(duration, Duration::from_millis(250));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
