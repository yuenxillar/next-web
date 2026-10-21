use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use next_web_core::any::AnyValue;

use crate::extended_state::{ExtendedState, ExtendedStateChangeListener};

/// Default implementation of an `ExtendedState`.
///
/// The variables live behind a [`RwLock`], so the machine which shares the
/// extended state can change them through a shared reference.
pub struct DefaultExtendedState {
    variables: RwLock<HashMap<String, AnyValue>>,
    listener: RwLock<Option<Arc<dyn ExtendedStateChangeListener>>>,
}

impl DefaultExtendedState {
    /// Instantiates a new default extended state with existing variables.
    ///
    /// # Arguments
    /// * `variables` - the variables the state starts with
    pub fn with_variables(variables: HashMap<String, AnyValue>) -> Self {
        Self {
            variables: RwLock::new(variables),
            listener: RwLock::new(None),
        }
    }
}

impl ExtendedState for DefaultExtendedState {
    fn get_variables(&self) -> HashMap<String, AnyValue> {
        self.variables
            .read()
            .map(|variables| variables.clone())
            .unwrap_or_default()
    }

    fn get(&self, key: &String) -> Option<AnyValue> {
        self.variables
            .read()
            .ok()
            .and_then(|variables| variables.get(key).cloned())
    }

    fn set(&self, key: String, value: AnyValue) {
        let Ok(mut variables) = self.variables.write() else {
            return;
        };
        variables.insert(key.clone(), value.clone());

        let listener = self
            .listener
            .read()
            .ok()
            .and_then(|listener| listener.clone());

        if let Some(listener) = listener {
            notify_change(&listener, &key, &value);
        }
    }

    fn set_extended_state_change_listener(
        &self,
        listener: Arc<dyn ExtendedStateChangeListener>,
    ) {
        if let Ok(mut current) = self.listener.write() {
            *current = Some(listener);
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
        let variables = self.get_variables();

        write!(f, "DefaultExtendedState [variables={variables:?}]")
    }
}

impl Default for DefaultExtendedState {
    fn default() -> Self {
        Self {
            variables: RwLock::new(HashMap::new()),
            listener: RwLock::new(None),
        }
    }
}
