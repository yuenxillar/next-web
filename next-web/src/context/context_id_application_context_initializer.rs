use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use next_web_context::ApplicationContextExt;
use next_web_core::{env::ConfigurableEnvironment, Ordered};

use crate::{ApplicationContextInitializer, ConfigurableApplicationContext};

/// `ApplicationContextInitializer` that sets the application context id.
///
/// The `spring.application.name` property is used to build the id. If the property
/// is not set, `application` is used instead.
pub struct ContextIdApplicationContextInitializer {
    order: i32,
}

impl ContextIdApplicationContextInitializer {
    /// Creates a new initializer with the default order
    /// (`i32::MAX - 10`).
    pub fn new() -> Self {
        Self {
            order: i32::MAX - 10,
        }
    }

    /// Overrides the ordering value.
    pub fn set_order(&mut self, order: i32) {
        self.order = order;
    }

    fn get_context_id(
        &self,
        application_context: &dyn ConfigurableApplicationContext,
    ) -> ContextId {
        if let Some(parent) = application_context.parent() {
            if parent.contains_singleton::<ContextId>() {
                return parent
                    .get_singleton_option_with_default_name::<ContextId>()
                    .map(|id| id.create_child_id())
                    .unwrap_or_else(|| {
                        ContextId::new(self.get_application_id(application_context.environment()))
                    });
            }
        }
        ContextId::new(self.get_application_id(application_context.environment()))
    }

    fn get_application_id(&self, environment: &dyn ConfigurableEnvironment) -> String {
        match environment.get_property("next.application.name") {
            Some(name) if !name.trim().is_empty() => name,
            _ => "application".to_owned(),
        }
    }
}

impl ApplicationContextInitializer for ContextIdApplicationContextInitializer {
    fn initialize(&mut self, application_context: &mut dyn ConfigurableApplicationContext) {
        let context_id = self.get_context_id(application_context);
        application_context.set_id(context_id.get_id().to_owned());
        application_context.insert_singleton_with_default_name(context_id);
    }
}

impl Ordered for ContextIdApplicationContextInitializer {
    fn order(&self) -> i32 {
        self.order
    }
}

impl Default for ContextIdApplicationContextInitializer {
    fn default() -> Self {
        Self::new()
    }
}

/// The id of a context.
#[derive(Clone)]
pub(crate) struct ContextId {
    children: Arc<AtomicU64>,
    id: String,
}

impl ContextId {
    /// Creates a new root context id.
    pub fn new(id: String) -> Self {
        Self {
            children: Arc::new(AtomicU64::new(0)),
            id,
        }
    }

    /// Creates a child context id by appending an incrementing suffix.
    pub fn create_child_id(&self) -> ContextId {
        let child = self.children.fetch_add(1, Ordering::SeqCst) + 1;
        ContextId {
            children: Arc::new(AtomicU64::new(0)),
            id: format!("{}-{}", self.id, child),
        }
    }

    /// Returns the id.
    pub fn get_id(&self) -> &str {
        &self.id
    }
}
