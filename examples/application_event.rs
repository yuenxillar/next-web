//! The application event example.
//!
//! The example registers an application listener and publishes an event once
//! the application has started, so the listener reports the events it receives.

use std::any::{Any, TypeId};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use next_web::{
    Application, NextWebApplication,
    core::{ApplicationContext, BoxFuture, Ordered, traits::apply_router::ApplyRouter},
    macros::bind::singleton,
};
use next_web_context::{
    ApplicationContextExt, ApplicationEvent, ApplicationListener, event::ApplicationEventMulticaster,
};

/// Test application
#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

/// The event the listener of the example reads.
#[derive(Clone)]
pub struct TestEvent {
    /// The time the event was published at, in milliseconds.
    timestamp: u64,
}

impl TestEvent {
    /// Creates an event that holds the given time.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The time the event was published at, in milliseconds.
    pub fn new(timestamp: u64) -> Self {
        Self { timestamp }
    }
}

impl ApplicationEvent for TestEvent {
    /// Returns the time the event was published at, in milliseconds.
    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the event itself, which is its source.
    fn source(&self) -> &dyn Any {
        self
    }

    /// Returns the type of the event.
    fn event_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Returns the type of the source of the event.
    fn source_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// The listener that reports the events of the application.
#[singleton(binds = [Self::into_listener])]
#[derive(Clone, Default)]
pub struct TestListener;

impl TestListener {
    fn into_listener(self) -> Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>> {
        Arc::new(self)
    }
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for TestListener {
    /// Reports the event the application published.
    ///
    /// # Arguments
    ///
    /// * `event` - The event the application published.
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            println!("Timestamp: {}", event.timestamp());
        })
    }
}

/// Publishes the test event once the routers of the application are applied.
#[singleton(binds = [Self::into_apply_router])]
#[derive(Clone)]
pub struct TestEventPublisher;

impl TestEventPublisher {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl Ordered for TestEventPublisher {
    fn order(&self) -> i32 {
        0
    }
}

impl ApplyRouter for TestEventPublisher {
    /// Publishes the test event every second, and contributes no route of its
    /// own to the application.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the multicaster is read from.
    fn apply(&mut self, ctx: &mut dyn ApplicationContext) -> Router {
        let Some(multicaster) = ctx
            .get_singleton_option_with_default_name::<Arc<dyn ApplicationEventMulticaster>>()
            .map(Arc::clone)
        else {
            return Router::new();
        };

        tokio::spawn(async move {
            loop {
                let _ = multicaster.multicast_event(Box::new(TestEvent::new(now_millis())));

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        Router::new()
    }
}

/// Returns the current time in milliseconds.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
