//! The data model which describes states and transitions before they are built
//! into a running machine.
/// Data of one choice branch.
pub mod choice_data;
/// Data of the generic machine configuration.
pub mod configuration_data;
/// The default model of a machine.
pub mod default_state_machine_model;
/// Data of an entry point.
pub mod entry_data;
/// Data of an exit point.
pub mod exit_data;
/// Data of a history state.
pub mod history_data;
/// Data of one junction branch.
pub mod junction_data;
/// Data of one state.
pub mod state_data;
/// The model contract of a machine.
pub mod state_machine_model;
/// The factory contract of a machine model.
pub mod state_machine_model_factory;
/// Data of the states of a machine.
pub mod states_data;
/// Data of one transition.
pub mod transition_data;
/// Data of all transitions, including the pseudo state data.
pub mod transitions_data;
/// Verifiers which check a model before it is built.
pub mod verifier;
