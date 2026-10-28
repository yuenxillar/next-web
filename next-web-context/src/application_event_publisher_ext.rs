//! The convenience methods of the publisher of the application.

use std::sync::Arc;

use crate::{
    ApplicationEvent, ApplicationEventPublisher,
    application_event_publisher::BoxError,
};

/// Publishes an event without naming the shared value it is published as.
///
/// The methods are the ones an application uses: the event it publishes is a value
/// of its own type, and the publisher shares it with every listener.
///
/// ```rust,ignore
/// publisher.publish(OrderPlaced { id })?;
/// ```
///
/// A publisher that is itself an
/// [`ApplicationEventPublisher`](ApplicationEventPublisher) implements the trait, so
/// the only thing an application has to do is to bring it into scope:
///
/// ```rust,ignore
/// use next_web_context::ApplicationEventPublisherExt;
/// ```
pub trait ApplicationEventPublisherExt {
    /// Publishes the event, sharing it with every listener of the application.
    ///
    /// # Arguments
    ///
    /// * `event` - The event of the application.
    fn publish<T>(&self, event: T) -> Result<(), BoxError>
    where
        T: ApplicationEvent + 'static;

    /// Publishes an event that is already shared.
    ///
    /// This is what a listener that publishes the event it received does: the event
    /// is shared again instead of being copied.
    ///
    /// # Arguments
    ///
    /// * `event` - The shared event of the application.
    fn publish_shared(&self, event: Arc<dyn ApplicationEvent>) -> Result<(), BoxError>;
}

impl<P> ApplicationEventPublisherExt for P
where
    P: ApplicationEventPublisher + ?Sized,
{
    fn publish<T>(&self, event: T) -> Result<(), BoxError>
    where
        T: ApplicationEvent + 'static,
    {
        self.publish_event(Arc::new(event))
    }

    fn publish_shared(&self, event: Arc<dyn ApplicationEvent>) -> Result<(), BoxError> {
        self.publish_event(event)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        any::Any,
        sync::{Arc, Mutex},
    };

    use crate::{
        ApplicationEvent, ApplicationEventPublisher, application_event_publisher::BoxError,
    };

    use super::ApplicationEventPublisherExt;

    /// An event of the tests, which is not cloneable: the publisher shares the one
    /// value it was given.
    struct Ping {
        /// The identifier of the event of the test.
        id: u32,
    }

    impl ApplicationEvent for Ping {
        fn source(&self) -> &dyn Any {
            self
        }
    }

    /// The publisher of the tests: it keeps the events it is given.
    #[derive(Default)]
    struct RecordingPublisher {
        /// The events the publisher was given, as the addresses of their values.
        published: Mutex<Vec<(usize, u32)>>,
    }

    impl RecordingPublisher {
        /// Returns the identifiers of the events the publisher was given.
        fn published(&self) -> Vec<(usize, u32)> {
            self.published
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl ApplicationEventPublisher for RecordingPublisher {
        fn publish_event(&self, event: Arc<dyn ApplicationEvent>) -> Result<(), BoxError> {
            let id = (event.as_ref() as &dyn Any)
                .downcast_ref::<Ping>()
                .map(|ping| ping.id)
                .unwrap_or_default();

            self.published
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push((Arc::as_ptr(&event) as *const () as usize, id));

            Ok(())
        }
    }

    #[test]
    fn the_event_of_the_application_is_published_as_one_shared_value() {
        let publisher = RecordingPublisher::default();

        publisher.publish(Ping { id: 7 }).expect("the event is published");

        assert_eq!(publisher.published().len(), 1);
        assert_eq!(publisher.published()[0].1, 7, "the event is the one published");
    }

    #[test]
    fn an_event_that_is_already_shared_is_published_as_it_is() {
        let publisher = RecordingPublisher::default();
        let event: Arc<dyn ApplicationEvent> = Arc::new(Ping { id: 9 });
        let address = Arc::as_ptr(&event) as *const () as usize;

        publisher
            .publish_shared(Arc::clone(&event))
            .expect("the event is published");

        assert_eq!(
            publisher.published()[0].0, address,
            "the shared event is published without a copy of it"
        );
    }
}
