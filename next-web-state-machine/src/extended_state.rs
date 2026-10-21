//! This module provides the `ExtendedState` trait for managing extended state variables
//! in a state machine.
//!
//! Extended states are used to supplement state machines with variables. When extended
//! state is used, the complete condition of a state machine is a combination of its
//! state and extended state variables.

use std::{collections::HashMap, sync::Arc};

use next_web_core::{any::AnyValue, async_trait};

/// Extended states are used to supplement state machines with variables.
/// When extended state is used, the complete condition of a state machine
/// is a combination of its state and extended state variables.
///
/// # Type Parameters
/// * `K` - The type of keys used to identify variables (must be thread-safe)
///
/// # Example
/// ```
/// use std::collections::HashMap;
///
/// use next_web_state_machine::extended_state::ExtendedState;
/// use next_web_state_machine::support::default_extended_state::DefaultExtendedState;
///
/// let extended_state = DefaultExtendedState::with_variables(HashMap::new());
/// assert!(extended_state.get_variables().is_empty());
/// ```
pub trait ExtendedState<K = String>
where
    Self: Send + Sync,
{
    /// Gets all extended state variables.
    ///
    /// # Returns
    /// A reference to the map containing all variables
    fn get_variables(&self) -> &HashMap<K, AnyValue>;

    /// Gets a variable with automatic type casting.
    fn get(&self, key: &K) -> Option<&AnyValue>;

    /// Sets the extended state change listener.
    fn set_extended_state_change_listener(
        &mut self,
        listener: Arc<dyn ExtendedStateChangeListener>,
    );
}

/// Listener notified when an extended state variable changes.
#[async_trait]
pub trait ExtendedStateChangeListener
where
    Self: Send + Sync,
    Self: 'static,
{
    /// Called when an extended state variable has been changed.
    ///
    /// # Arguments
    /// * `key` - The key of the changed variable
    /// * `value` - The new value of the variable
    async fn changed(&self, key: &str, value: &AnyValue);
}
