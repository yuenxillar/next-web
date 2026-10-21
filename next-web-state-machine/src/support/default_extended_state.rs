use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use next_web_core::any::AnyValue;

use crate::{
    extended_state::{ExtendedState, ExtendedStateChangeListener},
    support::observable_map::{MapChangeListener, ObservableMap},
};

/// Default implementation of an `ExtendedState`.
#[derive(Default)]
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
struct LocalMapChangeListener {
    listener: Option<Arc<dyn ExtendedStateChangeListener>>,
}

impl LocalMapChangeListener {
    fn new(listener: Arc<dyn ExtendedStateChangeListener>) -> Self {
        Self {
            listener: Some(listener),
        }
    }
}

impl MapChangeListener<String, AnyValue> for LocalMapChangeListener {
    fn added(&self, key: String, value: AnyValue) {
        if let Some(listener) = &self.listener {
            notify_change(listener, &key, &value);
        }
    }

    fn changed(&self, key: &String, value: &AnyValue) {
        if let Some(listener) = &self.listener {
            notify_change(listener, key, value);
        }
    }

    fn removed(&self, key: &String, value: &AnyValue) {
        if let Some(listener) = &self.listener {
            notify_change(listener, key, value);
        }
    }
}

/// Drives the future of the change listener.
///
/// The listener is async, so the future has to be polled; without a runtime the
/// change is only logged, because blocking here would deadlock an async caller.
fn notify_change(listener: &Arc<dyn ExtendedStateChangeListener>, key: &str, value: &AnyValue) {
    let listener = listener.clone();
    let key = key.to_owned();
    let value = value.clone();

    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn(async move {
                listener.changed(&key, &value).await;
            });
        }
        Err(_) => {
            tracing::debug!(
                key = key,
                "no runtime is available to notify the extended state change listener"
            );
        }
    }
}

impl fmt::Debug for DefaultExtendedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DefaultExtendedState [variables={:?}]", self.variables)
    }
}

