//! Event publishing shared by the resilience patterns.
//!
//! Every pattern reports what it does through a broadcast channel: state
//! transitions, rejections, slow calls, retries. Publishing never blocks the
//! guarded call and never allocates when nobody is listening, so an
//! application that does not subscribe pays nothing for the feature.

use tokio::sync::broadcast;

/// Number of events a subscriber may lag behind before it starts missing them.
///
/// Resilience instrumentation is a side channel: dropping events for a slow
/// subscriber is preferable to slowing down the calls being guarded.
pub(crate) const DEFAULT_EVENT_CHANNEL_CAPACITY: usize = 256;

/// Fan-out publisher for one pattern's events.
#[derive(Debug)]
pub(crate) struct EventPublisher<E> {
    sender: broadcast::Sender<E>,
}

impl<E> EventPublisher<E>
where
    E: Clone + Send + 'static,
{
    /// Creates a publisher with the default channel capacity.
    pub(crate) fn new() -> Self {
        Self::with_capacity(DEFAULT_EVENT_CHANNEL_CAPACITY)
    }

    /// Creates a publisher whose channel holds `capacity` events.
    ///
    /// A capacity of zero would make the channel unusable, so it is raised to
    /// one.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    /// Publishes `event` to every current subscriber.
    ///
    /// The event is dropped when nobody is subscribed; that is not an error,
    /// because instrumentation is optional.
    pub(crate) fn publish(&self, event: E) {
        // `send` only fails when there is no receiver left.
        let _ = self.sender.send(event);
    }

    /// Subscribes to the events published from now on.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<E> {
        self.sender.subscribe()
    }
}

impl<E> Clone for EventPublisher<E> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<E> Default for EventPublisher<E>
where
    E: Clone + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::EventPublisher;

    #[tokio::test]
    async fn subscribers_receive_published_events() {
        let publisher = EventPublisher::new();
        let mut receiver = publisher.subscribe();

        publisher.publish(7u32);

        assert_eq!(receiver.recv().await.ok(), Some(7));
    }

    #[tokio::test]
    async fn publishing_without_subscribers_is_not_an_error() {
        let publisher: EventPublisher<u32> = EventPublisher::new();

        publisher.publish(1);
        publisher.publish(2);
    }

    #[tokio::test]
    async fn cloning_a_publisher_shares_the_channel() {
        let publisher = EventPublisher::new();
        let mut receiver = publisher.subscribe();

        publisher.clone().publish(11u32);

        assert_eq!(receiver.recv().await.ok(), Some(11));
    }
}
