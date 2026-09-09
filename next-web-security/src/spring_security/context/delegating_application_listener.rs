use std::sync::Arc;

use next_web_context::{event::SmartApplicationListener, ApplicationEvent, ApplicationListener};
use next_web_core::BoxFuture;

/// Used for delegating to a number of `SmartApplicationListener` instances. This is
/// useful when needing to register a `SmartApplicationListener` with the
/// `ApplicationContext` programmatically.
#[derive(Clone)]
pub struct DelegatingApplicationListener {
    listeners: Vec<Arc<dyn SmartApplicationListener>>,
}

impl DelegatingApplicationListener {
    /// Adds a new `SmartApplicationListener` to use.
    ///
    /// # Arguments
    ///
    /// * `smart_application_listener` - The `SmartApplicationListener` to use. Cannot be
    ///   null.
    pub fn add_listener(&mut self, smart_application_listener: Arc<dyn SmartApplicationListener>) {
        self.listeners.push(smart_application_listener);
    }
}

impl Default for DelegatingApplicationListener {
    fn default() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for DelegatingApplicationListener {
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            for listener in self.listeners.iter() {
                // Check if the listener supports this event type.
                if !listener.supports_event_type(event.event_type())
                    && !listener.supports_source_type(event.source_type())
                {
                    // Notify the listener.
                    listener.on_application_event(event.clone()).await;
                }
            }
        })
    }
}
