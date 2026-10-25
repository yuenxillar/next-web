//! Lease bookkeeping: how long an acquisition stays valid and when it renews.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// The lease an acquisition was granted, and whether it is kept alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lease {
    /// How long Redis keeps the lock without further renewals.
    pub duration: Duration,
    /// Whether a watchdog renews the lease instead of letting it expire.
    pub watchdog: bool,
}

impl Lease {
    /// A lease kept alive by a watchdog.
    pub fn watchdog(duration: Duration) -> Self {
        Self {
            duration,
            watchdog: true,
        }
    }

    /// A lease that expires on its own after `duration`.
    pub fn fixed(duration: Duration) -> Self {
        Self {
            duration,
            watchdog: false,
        }
    }

    /// How often a watchdog renews this lease.
    ///
    /// Redisson renews at a third of the watchdog timeout, which leaves two
    /// attempts of headroom before the key expires. The floor keeps a very short
    /// lease from turning the watchdog into a busy loop.
    pub fn renewal_interval(&self) -> Duration {
        (self.duration / 3).max(Duration::from_millis(1))
    }
}

/// One node's contribution to an acquisition.
#[derive(Debug, Clone)]
pub struct NodeLease {
    /// The position of the node this lease belongs to.
    pub index: usize,
    /// The lease granted on that node.
    pub lease: Lease,
    /// Set by the watchdog when it can no longer prove ownership.
    pub lost: Arc<AtomicBool>,
}

impl NodeLease {
    /// Record a lease on node `index`.
    pub fn new(index: usize, lease: Lease) -> Self {
        Self {
            index,
            lease,
            lost: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Whether the watchdog has lost this lease.
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    /// Mark this lease as lost, which the guard reports as [`super::error::RedisLockError::WatchdogLost`].
    pub fn mark_lost(&self) {
        self.lost.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renewal_is_a_third_of_the_lease_with_a_one_millisecond_floor() {
        assert_eq!(
            Lease::watchdog(Duration::from_secs(30)).renewal_interval(),
            Duration::from_secs(10)
        );
        assert_eq!(
            Lease::watchdog(Duration::from_millis(2)).renewal_interval(),
            Duration::from_millis(1)
        );
        assert_eq!(
            Lease::fixed(Duration::from_millis(2)).renewal_interval(),
            Duration::from_millis(1)
        );
    }

    #[test]
    fn a_lease_starts_held_and_can_be_marked_lost() {
        let lease = NodeLease::new(0, Lease::watchdog(Duration::from_secs(30)));
        assert!(!lease.is_lost());

        lease.mark_lost();
        assert!(lease.is_lost());
    }
}
