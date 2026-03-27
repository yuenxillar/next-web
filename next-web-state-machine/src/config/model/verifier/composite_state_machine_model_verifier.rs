use std::{fmt::Debug, sync::Arc};

use next_web_core::error::BoxError;

use crate::config::model::{
    state_machine_model::StateMachineModel,
    verifier::{
        base_structure_verifier::BaseStructureVerifier,
        state_machine_model_verifier::StateMachineModelVerifier,
    },
};

/// Composite implementation of a state machine model verifier backed by multiple verifiers.
///
/// # Generic Parameters
/// * `S` - The type of state identifier
/// * `E` - The type of event
pub struct CompositeStateMachineModelVerifier<S, E> {
    verifiers: Vec<Arc<dyn StateMachineModelVerifier<S, E>>>,
}

impl<S, E> CompositeStateMachineModelVerifier<S, E>
where
    S: 'static,
    E: 'static,
{
    /// Registers a new verifier.
    ///
    /// # Parameters
    /// * `verifier` - The verifier to register
    pub fn register(&mut self, verifier: Arc<dyn StateMachineModelVerifier<S, E>>) {
        self.verifiers.push(verifier);
    }

    /// Gets all registered verifiers in reverse order (for iteration).
    fn verifiers_reverse(&self) -> impl Iterator<Item = &dyn StateMachineModelVerifier<S, E>> {
        self.verifiers.iter().map(AsRef::as_ref).rev()
    }
}

impl<S, E> Default for CompositeStateMachineModelVerifier<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Debug + PartialEq + Clone,
    E: Clone,
{
    fn default() -> Self {
        let mut verifier = Self {
            verifiers: Default::default(),
        };
        // Register base structure verifier
        verifier.register(Arc::new(BaseStructureVerifier::<S, E>::default()));
        verifier
    }
}

impl<S, E> StateMachineModelVerifier<S, E> for CompositeStateMachineModelVerifier<S, E>
where
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    /// Verifies the state machine model by executing all registered verifiers in reverse order.
    ///
    /// # Parameters
    /// * `model` - The state machine model to verify
    ///
    /// # Note
    /// Verifiers are executed in reverse order (LIFO) as they were registered.
    fn verify(&self, model: &dyn StateMachineModel<S, E>) -> Result<(), BoxError> {
        for verifier in self.verifiers_reverse() {
            let _ = verifier.verify(model);
        }
        Ok(())
    }
}
