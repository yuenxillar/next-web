//! Tests of the metadata an event reports and of the filtering the multicaster
//! does with it.

use std::{
    any::{Any, TypeId},
    sync::{Arc, Mutex},
};

use crate::{
    ApplicationEvent, ApplicationListener, BoxFuture,
    event::{
        ApplicationEventMulticaster, ApplicationListenerRegistration, ChannelDispatch,
        DefaultApplicationEventMulticaster, ListenerDispatch, TypedApplicationListener,
    },
};

/// An event that declares nothing but its source, which is itself.
///
/// The type of the event and the type of its source are the defaults of the trait,
/// so this event is what an event that writes no method of its own looks like.
#[derive(Clone, Debug)]
struct Ping;

impl ApplicationEvent for Ping {
    fn source(&self) -> &dyn Any {
        self
    }
}

/// Another event, so that a listener of `Ping` has an event it is not called for.
#[derive(Clone, Debug)]
struct Pong;

impl ApplicationEvent for Pong {
    fn source(&self) -> &dyn Any {
        self
    }
}

/// The listener of the tests: it records the names of the events it receives.
#[derive(Default)]
struct Recorder {
    received: Mutex<Vec<String>>,
}

impl Recorder {
    /// Returns what the listener received.
    fn received(&self) -> Vec<String> {
        self.received
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for Recorder {
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let name = if (event.as_ref() as &dyn Any).downcast_ref::<Ping>().is_some() {
                "ping"
            } else if (event.as_ref() as &dyn Any)
                .downcast_ref::<Pong>()
                .is_some()
            {
                "pong"
            } else {
                "other"
            };

            self.received
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(name.to_string());
        })
    }
}

/// The listener of the pings, which the adapter of the event type delivers.
struct PingRecorder {
    recorder: Arc<Recorder>,
}

impl ApplicationListener<Ping> for PingRecorder {
    fn on_application_event<'a>(&'a self, _event: Ping) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            self.recorder
                .received
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(String::from("ping"));
        })
    }
}

/// Returns a multicaster of the tests.
fn multicaster() -> Arc<DefaultApplicationEventMulticaster> {
    Arc::new(DefaultApplicationEventMulticaster::new_boxed())
}

#[test]
fn an_event_reports_the_type_of_itself_without_writing_a_method() {
    let ping = Ping;

    assert_eq!(ping.event_type(), TypeId::of::<Ping>());
    assert_eq!(ping.source_type(), TypeId::of::<Ping>());
    assert_eq!(ping.timestamp(), 0, "an event without a time reports none");
}

#[test]
fn the_type_an_event_reports_is_the_type_it_is_downcast_to() {
    // This is the pair that has to agree: the multicaster filters with
    // `event_type`, and the adapter of a listener downcasts with the type of the
    // value. An event that overrode `event_type` would pass the filter and then be
    // ignored by the listener.
    let event: Box<dyn ApplicationEvent> = Box::new(Ping);

    assert_eq!(event.event_type(), (*event).type_id());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_listener_is_only_called_for_the_event_type_it_is_registered_with() {
    let multicaster = multicaster();
    let recorder = Arc::new(Recorder::default());
    let listener = TypedApplicationListener::<PingRecorder, Ping>::new(PingRecorder {
        recorder: Arc::clone(&recorder),
    });

    multicaster.register(ApplicationListenerRegistration::new(
        "recorder",
        Arc::new(listener),
    ));
    multicaster
        .multicast_event(Box::new(Ping))
        .expect("the ping is published");
    multicaster
        .multicast_event(Box::new(Pong))
        .expect("the pong is published");

    assert_eq!(recorder.received(), vec![String::from("ping")]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_listener_of_a_source_type_is_only_called_for_the_events_of_it() {
    let multicaster = multicaster();
    let recorder = Arc::new(Recorder::default());
    let listener: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>> = recorder.clone();

    multicaster.register(
        ApplicationListenerRegistration::new("recorder", listener).with_source_type::<Ping>(),
    );
    multicaster
        .multicast_event(Box::new(Ping))
        .expect("the ping is published");
    multicaster
        .multicast_event(Box::new(Pong))
        .expect("the pong is published");

    assert_eq!(
        recorder.received(),
        vec![String::from("ping")],
        "the event of another source is not delivered"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_listener_of_another_source_type_receives_nothing() {
    let multicaster = multicaster();
    let recorder = Arc::new(Recorder::default());
    let listener: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>> = recorder.clone();

    multicaster.register(
        ApplicationListenerRegistration::new("recorder", listener).with_source_type::<String>(),
    );
    multicaster
        .multicast_event(Box::new(Ping))
        .expect("the ping is published");

    assert!(recorder.received().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_channel_listener_without_an_identifier_gets_a_queue_of_its_own() {
    let multicaster = multicaster();

    // The identifier is what a channel dispatched listener is keyed by: two
    // listeners registered without one have to end up with two queues.
    for _ in 0..2 {
        let listener: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>> =
            Arc::new(Recorder::default());

        multicaster.register(
            ApplicationListenerRegistration::new("", listener)
                .with_dispatch(ListenerDispatch::Channel(ChannelDispatch::with_capacity(4))),
        );
    }

    let ids = multicaster.channel_listener_ids();
    assert_eq!(ids.len(), 2, "every listener has a queue of its own: {ids:?}");
    assert!(
        ids.iter().all(|id| !id.is_empty()),
        "the identifiers an anonymous listener is given are not empty: {ids:?}"
    );
    assert_ne!(ids[0], ids[1]);

    multicaster.close_async().await;
}
