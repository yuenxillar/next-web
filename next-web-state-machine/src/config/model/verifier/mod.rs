//! Verifiers which check a model before it is built.
/// Verifier of the base structure of a model.
pub mod base_structure_verifier;
/// Verifier which dispatches to a set of verifiers.
pub mod composite_state_machine_model_verifier;
/// The default verifier of a model.
pub mod default_state_machine_model_verifier;
/// The verifier contract.
pub mod state_machine_model_verifier;
