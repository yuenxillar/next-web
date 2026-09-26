use std::{
    fmt::{self, Debug},
    panic::AssertUnwindSafe,
    sync::{Arc, RwLock},
};

use tokio::runtime::{Handle, RuntimeFlavor};

use crate::{
    ApplicationEvent, ApplicationListener,
    event::{
        ApplicationEventMulticaster, ApplicationListenerRegistration, ErasedApplicationListener,
        ListenerFailure, MulticastError,
        execution::{CatchUnwind, block_on, panic_message},
    },
};

/// Decides when the listeners of an event run relative to the call that
/// publishes the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DispatchMode {
    /// Runs the listeners as part of the publishing call.
    ///
    /// The call returns once every listener has run, so a caller that publishes
    /// an event knows the listeners have seen it. This is what Spring calls a
    /// synchronous multicaster, and it is the default.
    #[default]
    Inline,

    /// Hands the listeners to the runtime and returns without waiting for them.
    ///
    /// The listeners run in the order they were registered, on a task of the
    /// runtime the call was made on. This is the mode to use when a listener is
    /// slow and the caller should not wait for it. It requires a runtime, and
    /// it makes the failures of a listener unreachable by the caller, which is
    /// why they are always reported to the error handler.
    Spawned,
}

/// Decides what a multicaster returns when a listener of the event failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListenerFailurePolicy {
    /// Reports the failure to the error handler and reports success.
    ///
    /// A listener that fails is a problem of that listener, not of the
    /// publishing code, which is why this is the default: one listener must not
    /// be able to keep the others from running, nor interrupt the caller.
    #[default]
    Report,

    /// Reports the failure to the error handler and returns it.
    ///
    /// The listeners still all run; only the result of the call changes. Use
    /// this to make the code that publishes an event handle a listener failure.
    Propagate,
}

/// Receives the failures of the listeners of an event.
///
/// A listener reports a failure by panicking, which the multicaster catches so
/// that the remaining listeners still run. The handler is where that failure
/// ends up: the default one logs it, and an application that has a place to
/// report errors of this kind, such as a metrics or an alerting pipeline,
/// replaces it.
pub trait MulticastErrorHandler
where
    Self: Send + Sync,
{
    /// Handles the failure of a listener.
    ///
    /// # Arguments
    ///
    /// * `listener_id` - The identifier the listener was registered under.
    /// * `message` - The description of the failure.
    fn handle_listener_failure(&self, listener_id: &str, message: &str);
}

/// The error handler a multicaster uses when no other one is installed.
///
/// It writes the failure to the log of the application, so that a listener that
/// fails is visible without the publishing code having to handle it.
#[derive(Debug, Default, Clone, Copy)]
pub struct LoggingMulticastErrorHandler;

impl MulticastErrorHandler for LoggingMulticastErrorHandler {
    fn handle_listener_failure(&self, listener_id: &str, message: &str) {
        tracing::error!(
            listener = listener_id,
            error = message,
            "an application event listener failed"
        );
    }
}

/// The multicaster of the framework.
///
/// It keeps the listeners of an application, dispatches the events to the
/// listeners that support them, and isolates a listener that fails from the
/// listeners that follow it. See [`ApplicationEventMulticaster`] for the
/// operations of the trait and [`DispatchMode`] for when the listeners run.
///
/// The set of listeners is shared by every clone of a multicaster, so a clone
/// that is registered in an application context still observes the listeners
/// that are added to, or removed from, another clone.
#[derive(Clone)]
pub struct DefaultApplicationEventMulticaster {
    listeners: Arc<RwLock<Vec<ApplicationListenerRegistration>>>,
    error_handler: Arc<dyn MulticastErrorHandler>,
    dispatch_mode: DispatchMode,
    failure_policy: ListenerFailurePolicy,
}

impl DefaultApplicationEventMulticaster {
    /// Creates a multicaster without listeners.
    ///
    /// The multicaster dispatches events inline and reports the failures of its
    /// listeners to the log of the application.
    pub fn new_boxed() -> Self {
        Self {
            listeners: Arc::default(),
            error_handler: Arc::new(LoggingMulticastErrorHandler),
            dispatch_mode: DispatchMode::default(),
            failure_policy: ListenerFailurePolicy::default(),
        }
    }

    /// Creates a multicaster with the given listeners.
    ///
    /// The listeners receive every event. They are identified by their
    /// position, so a listener that an application registers later replaces one
    /// of them by name instead.
    ///
    /// # Arguments
    ///
    /// * `listeners` - The listeners to register, in the order they run in.
    pub fn new<I>(listeners: I) -> Self
    where
        I: IntoIterator<Item = ErasedApplicationListener>,
    {
        let multicaster = Self::new_boxed();

        for (index, listener) in listeners.into_iter().enumerate() {
            multicaster.register(ApplicationListenerRegistration::new(
                format!("listener-{index}"),
                listener,
            ));
        }

        multicaster
    }

    /// Installs the error handler that receives the failure of a listener.
    ///
    /// # Arguments
    ///
    /// * `error_handler` - The handler of the listener failures.
    pub fn with_error_handler(mut self, error_handler: Arc<dyn MulticastErrorHandler>) -> Self {
        self.error_handler = error_handler;
        self
    }

    /// Sets the error handler that receives the failure of a listener.
    ///
    /// # Arguments
    ///
    /// * `error_handler` - The handler of the listener failures.
    pub fn set_error_handler(&mut self, error_handler: Arc<dyn MulticastErrorHandler>) {
        self.error_handler = error_handler;
    }

    /// Sets when the listeners of an event run.
    ///
    /// # Arguments
    ///
    /// * `dispatch_mode` - The mode to dispatch in.
    pub fn with_dispatch_mode(mut self, dispatch_mode: DispatchMode) -> Self {
        self.dispatch_mode = dispatch_mode;
        self
    }

    /// Returns when the listeners of an event run.
    pub fn dispatch_mode(&self) -> DispatchMode {
        self.dispatch_mode
    }

    /// Sets what the multicaster does with a listener that failed.
    ///
    /// # Arguments
    ///
    /// * `failure_policy` - The policy to apply.
    pub fn with_failure_policy(mut self, failure_policy: ListenerFailurePolicy) -> Self {
        self.failure_policy = failure_policy;
        self
    }

    /// Returns what the multicaster does with a listener that failed.
    pub fn failure_policy(&self) -> ListenerFailurePolicy {
        self.failure_policy
    }

    /// Registers the listener, replacing the listener registered under the same
    /// identifier.
    ///
    /// # Arguments
    ///
    /// * `registration` - The listener and the metadata it is dispatched with.
    pub fn register(&self, registration: ApplicationListenerRegistration) {
        let mut listeners = self.write_listeners();

        match listeners
            .iter_mut()
            .find(|registered| registered.id() == registration.id())
        {
            Some(registered) => *registered = registration,
            None => listeners.push(registration),
        }
    }

    /// Registers a listener of one concrete event type.
    ///
    /// The listener is called for the events of `E` only, and it runs in the
    /// order of its registration among the listeners of the same order.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier the listener is registered under.
    /// * `listener` - The listener of the concrete event type.
    pub fn register_typed<L, E>(&self, id: impl Into<String>, listener: L)
    where
        L: ApplicationListener<E> + 'static,
        E: ApplicationEvent + 'static,
    {
        self.register(ApplicationListenerRegistration::typed(id, listener));
    }

    /// Returns whether a listener is registered under `id`.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the listener.
    pub fn contains_listener(&self, id: &str) -> bool {
        self.read_listeners()
            .iter()
            .any(|registered| registered.id() == id)
    }

    /// Returns the identifiers of the registered listeners, in the order they
    /// run in.
    pub fn listener_ids(&self) -> Vec<String> {
        self.ordered_listeners()
            .iter()
            .map(|registration| registration.id().to_owned())
            .collect()
    }

    /// Returns the number of registered listeners.
    pub fn listener_count(&self) -> usize {
        self.read_listeners().len()
    }

    /// Returns whether no listener is registered.
    pub fn is_empty(&self) -> bool {
        self.read_listeners().is_empty()
    }

    /// Publishes the event to the listeners that support it, on the current
    /// task.
    ///
    /// This is the asynchronous counterpart of the `multicast_event` method of
    /// [`ApplicationEventMulticaster`]: the listeners are awaited instead of
    /// being driven by the calling thread, so an application that publishes an
    /// event from an asynchronous function uses it to avoid blocking.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to publish.
    pub async fn multicast_event_async(
        &self,
        event: Box<dyn ApplicationEvent>,
    ) -> Result<(), MulticastError> {
        let registrations = self.matching_listeners(&*event);

        dispatch_listeners(
            registrations,
            event,
            Arc::clone(&self.error_handler),
            self.failure_policy,
        )
        .await
    }

    /// Returns the listeners that support the event, in the order they run in.
    ///
    /// # Arguments
    ///
    /// * `event` - The event that is published.
    pub fn matching_listeners(
        &self,
        event: &dyn ApplicationEvent,
    ) -> Vec<ApplicationListenerRegistration> {
        let event_type = event.event_type();

        self.ordered_listeners()
            .into_iter()
            .filter(|registration| registration.supports(event_type))
            .collect()
    }

    /// Returns the registered listeners, ordered by the order they run in.
    ///
    /// The registration order decides between listeners of the same order, so
    /// the sort is stable.
    fn ordered_listeners(&self) -> Vec<ApplicationListenerRegistration> {
        let mut listeners = self.read_listeners().clone();
        listeners.sort_by_key(ApplicationListenerRegistration::order);
        listeners
    }

    /// Runs the listeners of the event on a task of the runtime.
    ///
    /// # Arguments
    ///
    /// * `registrations` - The listeners to run, in the order they run in.
    /// * `event` - The event that is published.
    fn spawn_listeners(
        &self,
        registrations: Vec<ApplicationListenerRegistration>,
        event: Box<dyn ApplicationEvent>,
    ) -> Result<(), MulticastError> {
        if Handle::try_current().is_err() {
            tracing::warn!(
                "no runtime is available to dispatch an application event asynchronously"
            );

            return Err(MulticastError::Other(String::from(
                "no runtime is available to dispatch the event asynchronously",
            )));
        }

        let error_handler = Arc::clone(&self.error_handler);
        let failure_policy = self.failure_policy;

        tokio::spawn(async move {
            if let Err(error) =
                dispatch_listeners(registrations, event, error_handler, failure_policy).await
            {
                // The failure of a listener is reported by the error handler of
                // the multicaster already; this reports the aggregate the
                // asynchronous dispatch cannot return to its caller.
                tracing::debug!(
                    error = %error,
                    "an asynchronous event dispatch completed with failures"
                );
            }
        });

        Ok(())
    }

    /// Runs the listeners of the event on the calling thread.
    ///
    /// # Arguments
    ///
    /// * `registrations` - The listeners to run, in the order they run in.
    /// * `event` - The event that is published.
    fn run_listeners(
        &self,
        registrations: &[ApplicationListenerRegistration],
        event: &Box<dyn ApplicationEvent>,
    ) -> Result<(), MulticastError> {
        block_on(dispatch_listeners(
            registrations.to_vec(),
            event.clone(),
            Arc::clone(&self.error_handler),
            self.failure_policy,
        ))
    }

    fn read_listeners(
        &self,
    ) -> std::sync::RwLockReadGuard<'_, Vec<ApplicationListenerRegistration>> {
        self.listeners
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_listeners(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, Vec<ApplicationListenerRegistration>> {
        self.listeners
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for DefaultApplicationEventMulticaster {
    fn default() -> Self {
        Self::new_boxed()
    }
}

impl ApplicationEventMulticaster for DefaultApplicationEventMulticaster {
    fn add_application_listener(&self, id: String, listener: ErasedApplicationListener) {
        self.register(ApplicationListenerRegistration::new(id, listener));
    }

    fn remove_application_listener(&self, id: String) {
        self.write_listeners()
            .retain(|registered| registered.id() != id);
    }

    fn remove_all_listeners(&self) {
        self.write_listeners().clear();
    }

    fn multicast_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), MulticastError> {
        let registrations = self.matching_listeners(&*event);

        if registrations.is_empty() {
            return Ok(());
        }

        match self.dispatch_mode {
            DispatchMode::Spawned => self.spawn_listeners(registrations, event),
            DispatchMode::Inline => match Handle::try_current() {
                // The runtime keeps making progress while this thread waits for
                // the listeners, and `block_in_place` tells it to move the
                // tasks of this thread to another one meanwhile.
                Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => {
                    let blocked = std::panic::catch_unwind(AssertUnwindSafe(|| {
                        tokio::task::block_in_place(|| self.run_listeners(&registrations, &event))
                    }));

                    // A `LocalSet` does not allow blocking either. The event is
                    // dispatched on the calling thread in that case, which is
                    // what a listener that does not await expects as well.
                    blocked.unwrap_or_else(|_| self.run_listeners(&registrations, &event))
                }
                // The single thread of a current thread runtime drives the
                // reactor of the runtime, so waiting for a listener on it would
                // keep a listener that awaits from ever making progress. The
                // listeners run on the runtime instead, and publishing does not
                // wait for them.
                Ok(_) => self.spawn_listeners(registrations, event),
                Err(_) => self.run_listeners(&registrations, &event),
            },
        }
    }
}

impl Debug for DefaultApplicationEventMulticaster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultApplicationEventMulticaster")
            .field("listener_ids", &self.listener_ids())
            .field("dispatch_mode", &self.dispatch_mode)
            .field("failure_policy", &self.failure_policy)
            .finish_non_exhaustive()
    }
}

/// Runs the listeners of an event in order, reporting the ones that panic.
///
/// # Arguments
///
/// * `registrations` - The listeners to run, in the order they run in.
/// * `event` - The event that is published.
/// * `error_handler` - The handler of the failures.
/// * `failure_policy` - What to return when a listener failed.
async fn dispatch_listeners(
    registrations: Vec<ApplicationListenerRegistration>,
    event: Box<dyn ApplicationEvent>,
    error_handler: Arc<dyn MulticastErrorHandler>,
    failure_policy: ListenerFailurePolicy,
) -> Result<(), MulticastError> {
    let mut failures = Vec::new();

    for registration in &registrations {
        let listener = registration.listener();
        let outcome = CatchUnwind::new(listener.on_application_event(event.clone())).await;

        if let Err(payload) = outcome {
            let message = panic_message(&payload);
            error_handler.handle_listener_failure(registration.id(), &message);
            failures.push(ListenerFailure::new(registration.id(), message));
        }
    }

    match failure_policy {
        ListenerFailurePolicy::Report => Ok(()),
        ListenerFailurePolicy::Propagate if failures.is_empty() => Ok(()),
        ListenerFailurePolicy::Propagate => Err(MulticastError::ListenerFailures(failures)),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        any::{Any, TypeId},
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use crate::{
        ApplicationEvent, ApplicationListener, BoxFuture,
        event::{
            ApplicationEventMulticaster, ApplicationListenerRegistration,
            DefaultApplicationEventMulticaster, DispatchMode, ErasedApplicationListener,
            ListenerFailurePolicy, MulticastError, MulticastErrorHandler,
        },
    };

    /// An event of the tests.
    #[derive(Clone, Debug)]
    struct TestEvent;

    impl ApplicationEvent for TestEvent {
        fn source(&self) -> &dyn Any {
            self
        }

        fn event_type(&self) -> TypeId {
            TypeId::of::<Self>()
        }

        fn source_type(&self) -> TypeId {
            TypeId::of::<Self>()
        }
    }

    /// Another event of the tests.
    #[derive(Clone, Debug)]
    struct OtherEvent;

    impl ApplicationEvent for OtherEvent {
        fn source(&self) -> &dyn Any {
            self
        }

        fn event_type(&self) -> TypeId {
            TypeId::of::<Self>()
        }

        fn source_type(&self) -> TypeId {
            TypeId::of::<Self>()
        }
    }

    /// A listener that records the events it receives.
    #[derive(Default)]
    struct RecordingListener {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl ApplicationListener<Box<dyn ApplicationEvent>> for RecordingListener {
        fn on_application_event<'a>(
            &'a self,
            event: Box<dyn ApplicationEvent>,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                let name = if (*event).type_id() == TypeId::of::<TestEvent>() {
                    "test"
                } else {
                    "other"
                };

                self.events.lock().unwrap().push(name);
            })
        }
    }

    /// A listener that panics.
    struct PanickingListener;

    impl ApplicationListener<Box<dyn ApplicationEvent>> for PanickingListener {
        fn on_application_event<'a>(
            &'a self,
            _event: Box<dyn ApplicationEvent>,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                panic!("the listener of the test failed");
            })
        }
    }

    /// A listener that records the position it ran in.
    struct OrderListener {
        name: &'static str,
        ran: Arc<Mutex<Vec<&'static str>>>,
    }

    impl ApplicationListener<Box<dyn ApplicationEvent>> for OrderListener {
        fn on_application_event<'a>(
            &'a self,
            _event: Box<dyn ApplicationEvent>,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                self.ran.lock().unwrap().push(self.name);
            })
        }
    }

    /// A listener that counts the events it receives.
    struct CountingListener {
        received: Arc<AtomicUsize>,
    }

    impl ApplicationListener<TestEvent> for CountingListener {
        fn on_application_event<'a>(&'a self, _event: TestEvent) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                self.received.fetch_add(1, Ordering::SeqCst);
            })
        }
    }

    /// A listener that awaits before it records the event.
    struct SlowListener {
        ran: Arc<AtomicUsize>,
        delay: Duration,
    }

    impl ApplicationListener<Box<dyn ApplicationEvent>> for SlowListener {
        fn on_application_event<'a>(
            &'a self,
            _event: Box<dyn ApplicationEvent>,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                tokio::time::sleep(self.delay).await;
                self.ran.fetch_add(1, Ordering::SeqCst);
            })
        }
    }

    /// An error handler that records the failures.
    #[derive(Default)]
    struct RecordingErrorHandler {
        failures: Mutex<Vec<String>>,
    }

    impl MulticastErrorHandler for RecordingErrorHandler {
        fn handle_listener_failure(&self, listener_id: &str, message: &str) {
            self.failures
                .lock()
                .unwrap()
                .push(format!("{listener_id}: {message}"));
        }
    }

    /// Creates a multicaster with a listener that records the events and with
    /// an error handler that records the failures.
    fn recording_multicaster(
        events: &Arc<Mutex<Vec<&'static str>>>,
    ) -> (
        DefaultApplicationEventMulticaster,
        Arc<RecordingErrorHandler>,
    ) {
        let error_handler = Arc::new(RecordingErrorHandler::default());
        let multicaster = DefaultApplicationEventMulticaster::default()
            .with_error_handler(error_handler.clone());

        let listener: ErasedApplicationListener = Arc::new(RecordingListener {
            events: Arc::clone(events),
        });
        multicaster.add_application_listener(String::from("recording"), listener);

        (multicaster, error_handler)
    }

    #[test]
    fn dispatches_an_event_to_the_registered_listener() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let (multicaster, _) = recording_multicaster(&events);

        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");

        assert_eq!(*events.lock().unwrap(), vec!["test"]);
    }

    #[test]
    fn replacing_a_listener_keeps_a_single_registration() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let (multicaster, _) = recording_multicaster(&events);

        let listener: ErasedApplicationListener = Arc::new(RecordingListener {
            events: Arc::clone(&events),
        });
        multicaster.add_application_listener(String::from("recording"), listener);

        assert_eq!(multicaster.listener_count(), 1);

        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");

        assert_eq!(*events.lock().unwrap(), vec!["test"]);
    }

    #[test]
    fn removing_a_listener_stops_the_events() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let (multicaster, _) = recording_multicaster(&events);

        multicaster.remove_application_listener(String::from("recording"));
        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");

        assert!(multicaster.is_empty());
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn removing_all_listeners_stops_the_events() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let (multicaster, _) = recording_multicaster(&events);

        multicaster.remove_all_listeners();
        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");

        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn an_event_without_listeners_is_discarded() {
        let multicaster = DefaultApplicationEventMulticaster::default();

        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");
    }

    #[test]
    fn a_typed_listener_receives_only_the_events_of_its_type() {
        let received = Arc::new(AtomicUsize::new(0));
        let multicaster = DefaultApplicationEventMulticaster::default();
        multicaster.register_typed(
            "counting",
            CountingListener {
                received: Arc::clone(&received),
            },
        );

        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");
        assert_eq!(received.load(Ordering::SeqCst), 1);

        multicaster
            .multicast_event(Box::new(OtherEvent))
            .expect("publishing succeeds");
        assert_eq!(received.load(Ordering::SeqCst), 1);

        assert_eq!(multicaster.matching_listeners(&TestEvent).len(), 1);
        assert!(multicaster.matching_listeners(&OtherEvent).is_empty());
    }

    #[test]
    fn a_listener_that_panics_does_not_stop_the_others() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let error_handler = Arc::new(RecordingErrorHandler::default());
        let multicaster = DefaultApplicationEventMulticaster::default()
            .with_error_handler(error_handler.clone());

        multicaster
            .add_application_listener(String::from("panicking"), Arc::new(PanickingListener));
        multicaster.add_application_listener(
            String::from("recording"),
            Arc::new(RecordingListener {
                events: Arc::clone(&events),
            }),
        );

        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("the failure is reported, not returned");

        assert_eq!(*events.lock().unwrap(), vec!["test"]);
        assert_eq!(error_handler.failures.lock().unwrap().len(), 1);
        assert!(error_handler.failures.lock().unwrap()[0].starts_with("panicking: "));
    }

    #[test]
    fn a_listener_that_panics_is_returned_when_the_policy_propagates() {
        let multicaster = DefaultApplicationEventMulticaster::default()
            .with_failure_policy(ListenerFailurePolicy::Propagate);
        multicaster
            .add_application_listener(String::from("panicking"), Arc::new(PanickingListener));

        let error = multicaster
            .multicast_event(Box::new(TestEvent))
            .expect_err("the failure is returned");

        match error {
            MulticastError::ListenerFailures(failures) => {
                assert_eq!(failures.len(), 1);
                assert_eq!(failures[0].listener_id, "panicking");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn listeners_run_in_the_order_they_declared() {
        let ran = Arc::new(Mutex::new(Vec::new()));
        let multicaster = DefaultApplicationEventMulticaster::default();

        for (id, order, name) in [
            ("last", 20, "last"),
            ("first", -10, "first"),
            ("middle", 0, "middle"),
        ] {
            let listener: ErasedApplicationListener = Arc::new(OrderListener {
                name,
                ran: Arc::clone(&ran),
            });
            multicaster
                .register(ApplicationListenerRegistration::new(id, listener).with_order(order));
        }

        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");

        assert_eq!(*ran.lock().unwrap(), vec!["first", "middle", "last"]);
    }

    #[test]
    fn a_clone_observes_the_listeners_of_another_clone() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let (multicaster, _) = recording_multicaster(&events);
        let clone = multicaster.clone();

        clone.remove_all_listeners();
        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");

        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_listener_that_awaits_has_run_when_publishing_returns() {
        let ran = Arc::new(AtomicUsize::new(0));
        let multicaster = DefaultApplicationEventMulticaster::default();
        multicaster.add_application_listener(
            String::from("slow"),
            Arc::new(SlowListener {
                ran: Arc::clone(&ran),
                delay: Duration::from_millis(20),
            }),
        );

        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");

        assert_eq!(ran.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_awaited_dispatch_runs_the_listeners()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let received = Arc::new(AtomicUsize::new(0));
        let multicaster = DefaultApplicationEventMulticaster::default();
        multicaster.register_typed(
            "counting",
            CountingListener {
                received: Arc::clone(&received),
            },
        );

        multicaster
            .multicast_event_async(Box::new(TestEvent))
            .await?;

        assert_eq!(received.load(Ordering::SeqCst), 1);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_spawned_dispatch_does_not_wait_for_the_listeners() {
        let ran = Arc::new(AtomicUsize::new(0));
        let multicaster = DefaultApplicationEventMulticaster::default()
            .with_dispatch_mode(DispatchMode::Spawned);
        multicaster.add_application_listener(
            String::from("slow"),
            Arc::new(SlowListener {
                ran: Arc::clone(&ran),
                delay: Duration::from_millis(50),
            }),
        );

        multicaster
            .multicast_event(Box::new(TestEvent))
            .expect("publishing succeeds");

        assert_eq!(ran.load(Ordering::SeqCst), 0);

        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(ran.load(Ordering::SeqCst), 1);
    }
}
