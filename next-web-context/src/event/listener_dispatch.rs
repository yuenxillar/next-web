//! How a listener is dispatched, and the queue a channel dispatched one uses.

use std::{
    fmt::{self, Debug},
    sync::atomic::{AtomicU64, Ordering},
};

/// How the events of a listener reach it.
///
/// A listener is dispatched inline by default: it runs as part of the call that
/// publishes the event, so the caller knows the listener has seen the event when
/// the call returns. A listener that is dispatched over a channel instead runs on
/// a worker of its own, one event after the other, and the call that publishes the
/// event only enqueues it.
///
/// # When to use which
///
/// Inline is the right dispatch for a listener whose effect the request depends
/// on: what it writes is read by the request that follows, or by the request that
/// publishes the event itself. The session registry of an application is the
/// example: a session that was destroyed has to be gone from the registry before
/// the next request is authorized.
///
/// A channel is the right dispatch for a listener whose effect the request does
/// not depend on and that is slow: a notification, a webhook, an audit trail that
/// is written somewhere else. It keeps a slow listener from slowing down the
/// request that published the event, and it keeps the listeners of the event from
/// waiting for each other. The price is the one every queue charges: the event is
/// delivered after the publishing call returned, the delivery can be refused when
/// the queue is full, and a listener that fails is only visible to the error
/// handler of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListenerDispatch {
    /// Runs the listener as part of the publishing call.
    #[default]
    Inline,
    /// Queues the event for the worker of the listener.
    Channel(ChannelDispatch),
}

impl ListenerDispatch {
    /// Returns whether the listener runs as part of the publishing call.
    pub fn is_inline(&self) -> bool {
        matches!(self, Self::Inline)
    }

    /// Returns whether the listener runs on a worker of its own.
    pub fn is_channel(&self) -> bool {
        matches!(self, Self::Channel(_))
    }

    /// Returns the queue of a listener that is dispatched over a channel.
    pub fn channel(&self) -> Option<&ChannelDispatch> {
        match self {
            Self::Inline => None,
            Self::Channel(channel) => Some(channel),
        }
    }
}

/// The queue of a listener that is dispatched over a channel.
///
/// The queue is bounded, and the call that publishes an event never waits for
/// room: a queue that is full refuses the event, which is reported to the error
/// handler of the multicaster and counted in
/// [`ChannelStatistics::rejected`]. A publisher that wants to wait for room has
/// to choose the dispatch of a listener that can, such as
/// [`ListenerDispatch::Inline`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelDispatch {
    capacity: usize,
    when_full: WhenFull,
}

impl ChannelDispatch {
    /// The number of events a queue holds when the listener does not ask for a
    /// capacity of its own.
    pub const DEFAULT_CAPACITY: usize = 1024;

    /// Creates the queue of a listener with the default capacity.
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    /// Creates the queue of a listener that holds `capacity` events.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The number of events the queue holds. A capacity of zero is
    ///   replaced by one, so that the queue can always be published to.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            when_full: WhenFull::default(),
        }
    }

    /// Sets what happens when the queue is full.
    ///
    /// # Arguments
    ///
    /// * `when_full` - The policy to apply.
    pub fn with_when_full(mut self, when_full: WhenFull) -> Self {
        self.when_full = when_full;
        self
    }

    /// Returns the number of events the queue holds.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns what happens when the queue is full.
    pub fn when_full(&self) -> WhenFull {
        self.when_full
    }
}

impl Default for ChannelDispatch {
    fn default() -> Self {
        Self::new()
    }
}

/// What a channel dispatched listener does when its queue is full.
///
/// The publishing call is synchronous, so it cannot wait for room without
/// blocking the thread it runs on: the only safe answer is to refuse the event,
/// which is what [`Self::Refuse`] does. The variant is named rather than implied
/// so that a policy that waits can be added without changing the meaning of the
/// dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhenFull {
    /// Refuses the event: it is reported to the error handler and counted.
    #[default]
    Refuse,
}

/// Counts what a channel dispatched listener received and what it lost.
///
/// The counters are shared with the worker of the listener and read through
/// [`DefaultApplicationEventMulticaster::channel_statistics`], so an application
/// that reports the health of its services can tell a listener that is behind
/// from one that is not receiving anything.
///
/// [`DefaultApplicationEventMulticaster::channel_statistics`]: crate::event::DefaultApplicationEventMulticaster::channel_statistics
#[derive(Debug, Default)]
pub struct ChannelStatistics {
    sent: AtomicU64,
    delivered: AtomicU64,
    rejected: AtomicU64,
    failed: AtomicU64,
}

impl ChannelStatistics {
    /// Returns the number of events the queue accepted.
    pub fn sent(&self) -> u64 {
        self.sent.load(Ordering::Relaxed)
    }

    /// Returns the number of events the listener has run for.
    pub fn delivered(&self) -> u64 {
        self.delivered.load(Ordering::Relaxed)
    }

    /// Returns the number of events the queue refused because it was full.
    pub fn rejected(&self) -> u64 {
        self.rejected.load(Ordering::Relaxed)
    }

    /// Returns the number of events whose listener failed.
    pub fn failed(&self) -> u64 {
        self.failed.load(Ordering::Relaxed)
    }

    /// Returns the number of events the queue accepted and the listener has not
    /// run for yet.
    pub fn pending(&self) -> u64 {
        self.sent().saturating_sub(self.delivered())
    }

    /// Returns the counters of a listener, as a snapshot that can be compared.
    pub(crate) fn snapshot(&self) -> ChannelStatisticsSnapshot {
        ChannelStatisticsSnapshot {
            sent: self.sent(),
            delivered: self.delivered(),
            rejected: self.rejected(),
            failed: self.failed(),
        }
    }

    /// Counts an event the queue accepted.
    pub(crate) fn record_sent(&self) {
        self.sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts an event the queue refused.
    pub(crate) fn record_rejected(&self) {
        self.rejected.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts an event the listener ran for.
    pub(crate) fn record_delivered(&self) {
        self.delivered.fetch_add(1, Ordering::Relaxed);
    }

    /// Counts an event whose listener failed.
    pub(crate) fn record_failed(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }
}

/// The counters of a channel dispatched listener at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelStatisticsSnapshot {
    /// The number of events the queue accepted.
    pub sent: u64,
    /// The number of events the listener has run for.
    pub delivered: u64,
    /// The number of events the queue refused because it was full.
    pub rejected: u64,
    /// The number of events whose listener failed.
    pub failed: u64,
}

impl ChannelStatisticsSnapshot {
    /// Returns the number of events the queue accepted and the listener has not
    /// run for yet.
    pub fn pending(&self) -> u64 {
        self.sent.saturating_sub(self.delivered)
    }
}

impl fmt::Display for ChannelStatisticsSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sent={} delivered={} pending={} rejected={} failed={}",
            self.sent,
            self.delivered,
            self.pending(),
            self.rejected,
            self.failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_queue_holds_at_least_one_event() {
        assert_eq!(ChannelDispatch::with_capacity(0).capacity(), 1);
        assert_eq!(ChannelDispatch::new().capacity(), ChannelDispatch::DEFAULT_CAPACITY);
        assert_eq!(ChannelDispatch::new().when_full(), WhenFull::Refuse);
    }

    #[test]
    fn an_inline_listener_has_no_queue() {
        let dispatch = ListenerDispatch::default();

        assert!(dispatch.is_inline());
        assert!(!dispatch.is_channel());
        assert!(dispatch.channel().is_none());
    }

    #[test]
    fn a_channel_listener_carries_its_queue() {
        let dispatch = ListenerDispatch::Channel(ChannelDispatch::with_capacity(8));

        assert!(dispatch.is_channel());
        assert_eq!(dispatch.channel().map(ChannelDispatch::capacity), Some(8));
    }

    #[test]
    fn the_counters_report_what_the_listener_received_and_lost() {
        let statistics = ChannelStatistics::default();
        statistics.record_sent();
        statistics.record_sent();
        statistics.record_delivered();
        statistics.record_rejected();
        statistics.record_failed();
        let snapshot = statistics.snapshot();

        assert_eq!(snapshot.sent, 2);
        assert_eq!(snapshot.delivered, 1);
        assert_eq!(snapshot.rejected, 1);
        assert_eq!(snapshot.failed, 1);
        assert_eq!(snapshot.pending(), 1);
        assert_eq!(
            snapshot.to_string(),
            "sent=2 delivered=1 pending=1 rejected=1 failed=1"
        );
    }
}
