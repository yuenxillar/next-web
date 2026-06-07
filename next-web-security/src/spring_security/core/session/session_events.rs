use std::sync::Arc;

use crate::core::context::SecurityContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstractSessionEvent {
    source: String,
}

impl AbstractSessionEvent {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

#[derive(Clone)]
pub struct SessionDestroyedEvent {
    base: AbstractSessionEvent,
    id: String,
    security_contexts: Vec<Arc<dyn SecurityContext>>,
}

impl SessionDestroyedEvent {
    pub fn new(
        source: impl Into<String>,
        id: impl Into<String>,
        security_contexts: Vec<Arc<dyn SecurityContext>>,
    ) -> Self {
        Self {
            base: AbstractSessionEvent::new(source),
            id: id.into(),
            security_contexts,
        }
    }

    pub fn source(&self) -> &str {
        self.base.source()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn security_contexts(&self) -> &[Arc<dyn SecurityContext>] {
        &self.security_contexts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionIdChangedEvent {
    base: AbstractSessionEvent,
    old_session_id: String,
    new_session_id: String,
}

impl SessionIdChangedEvent {
    pub fn new(
        source: impl Into<String>,
        old_session_id: impl Into<String>,
        new_session_id: impl Into<String>,
    ) -> Self {
        Self {
            base: AbstractSessionEvent::new(source),
            old_session_id: old_session_id.into(),
            new_session_id: new_session_id.into(),
        }
    }

    pub fn source(&self) -> &str {
        self.base.source()
    }

    pub fn old_session_id(&self) -> &str {
        &self.old_session_id
    }

    pub fn new_session_id(&self) -> &str {
        &self.new_session_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionCreationEvent {
    base: AbstractSessionEvent,
}

impl SessionCreationEvent {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            base: AbstractSessionEvent::new(source),
        }
    }

    pub fn source(&self) -> &str {
        self.base.source()
    }
}

#[derive(Clone)]
pub enum SessionEvent {
    Created(SessionCreationEvent),
    Destroyed(SessionDestroyedEvent),
    IdChanged(SessionIdChangedEvent),
}
