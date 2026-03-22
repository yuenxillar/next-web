use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;

use crate::state_machine::{
    extended_state::{ExtendedState, ExtendedStateChangeListener},
    support::observable_map::{MapChangeListener, ObservableMap},
};

/// Default implementation of an `ExtendedState`.
pub struct DefaultExtendedState {
    variables: ObservableMap<String, AnyValue>,
    listener: Option<Arc<dyn ExtendedStateChangeListener>>,
}

impl DefaultExtendedState {
    /// Instantiates a new default extended state with existing variables.
    pub fn with_variables(variables: HashMap<String, AnyValue>) -> Self {
        Self {
            variables: ObservableMap::new(variables),
            listener: None,
        }
    }
}

impl ExtendedState for DefaultExtendedState {
    fn get_variables(&self) -> &HashMap<String, AnyValue> {
        &self.variables.delegate
    }

    fn get(&self, key: &String) -> Option<&AnyValue> {
        self.variables.get(key)
    }

    fn set_extended_state_change_listener(
        &mut self,
        listener: Arc<dyn ExtendedStateChangeListener>,
    ) {
        self.listener = Some(listener.clone());
        // Register the listener with the observable map
        self.variables
            .listener
            .replace(Arc::new(LocalMapChangeListener::new(listener.clone())));
    }
}

#[derive(Default)]
pub(super) struct LocalMapChangeListener {
    listener: Option<Arc<dyn ExtendedStateChangeListener>>,
}

impl LocalMapChangeListener {
    pub fn new(listener: Arc<dyn ExtendedStateChangeListener>) -> Self {
        Self {
            listener: Some(listener),
        }
    }

    pub fn set_listener(&mut self, listener: Arc<dyn ExtendedStateChangeListener>) {
        self.listener = Some(listener);
    }
}

impl MapChangeListener<String, AnyValue> for LocalMapChangeListener {
    fn added(&self, key: String, value: AnyValue) {
        if let Some(listener) = &self.listener {
            listener.changed(&key, &value);
        }
    }

    fn changed(&self, key: &String, value: &AnyValue) {
        if let Some(listener) = &self.listener {
            listener.changed(key, value);
        }
    }

    fn removed(&self, key: &String, value: &AnyValue) {
        if let Some(listener) = &self.listener {
            listener.changed(key, value);
        }
    }
}

impl fmt::Debug for DefaultExtendedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DefaultExtendedState [variables={:?}]", self.variables)
    }
}

impl Default for DefaultExtendedState {
    fn default() -> Self {
        Self {
            variables: ObservableMap::default(),
            listener: None,
        }
    }
}
