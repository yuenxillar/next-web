use std::{any::TypeId, sync::Arc};

use crate::{
    ApplicationEvent, ApplicationListener, BoxFuture,
    event::{GenericApplicationListener, SmartApplicationListener},
};

/// GenericApplicationListener adapter that determines supported event types through introspecting
/// the generically declared type of the target listener.
pub struct GenericApplicationListenerAdapter {
    delegate: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
}

impl GenericApplicationListenerAdapter {
    // Create a new GenericApplicationListener for the given delegate
    pub fn new(delegate: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>) -> Self {
        Self { delegate }
    }
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for GenericApplicationListenerAdapter {
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async {
            self.delegate.on_application_event(event).await;
        })
    }
}

impl GenericApplicationListener for GenericApplicationListenerAdapter {
    fn supports_source_type(&self, _source_type: TypeId) -> bool {
        true
    }
}

impl SmartApplicationListener for GenericApplicationListenerAdapter {
    fn supports_event_type(&self, _event: TypeId) -> bool {
        true
    }

    fn order(&self) -> i32 {
        i32::MAX
    }

    fn listener_id(&self) -> &str {
        ""
    }
}
