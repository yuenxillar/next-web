use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use next_web_context::{ApplicationEvent, EventAttributes};
use next_web_core::traits::http::HttpSession;

use crate::core::context::SecurityContext;

/// Published by the HttpSessionEventPublisher when a HttpSession is removed from the container
#[derive(Clone)]
pub struct HttpSessionDestroyedEvent {
    inner: EventAttributes<Arc<dyn HttpSession>>,
}

impl HttpSessionDestroyedEvent {
    pub fn new(session: Arc<dyn HttpSession>) -> Self {
        Self {
            inner: EventAttributes::new(session),
        }
    }

    pub fn session(&self) -> &dyn HttpSession {
        self.base.value().as_ref()
    }

    pub fn get_security_contexts(&self) -> Vec<Arc<dyn SecurityContext>> {
        let session = self.session();
        let attributes = session.attribute_names();

        attributes
            .into_iter()
            .filter_map(|name| {
                session
                    .attribute(name)
                    .and_then(|value| value.as_object::<Arc<dyn SecurityContext>>())
            })
            .collect()
    }

    pub fn id(&self) -> &str {
        self.session().id()
    }
}

impl ApplicationEvent for HttpSessionDestroyedEvent {
    fn event_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn source_type(&self) -> TypeId {
        self.base.source_type()
    }

    fn source(&self) -> &dyn Any {
        self.base.source()
    }

    fn timestamp(&self) -> u64 {
        self.base.timestamp()
    }
}
