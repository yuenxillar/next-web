//! Support types and the retry template itself.
//!
//! [`retry_template::RetryTemplate`] is the entry point of the crate: it wires a retry policy,
//! a back-off policy and optional listeners into an executable operation. The remaining
//! modules provide the pieces the template and the interceptors need: retry state, argument
//! capture, the task-local current context, metrics reporting and offline simulation of a
//! schedule.

pub mod args;
pub mod default_retry_state;
pub mod metrics_retry_listener;
pub mod retry_metrics;
pub mod retry_simulation;
pub mod retry_simulator;
pub mod retry_synchronization_manager;
pub mod retry_template;
pub mod retry_template_builder;

pub use args::Args;
pub use default_retry_state::DefaultRetryState;
pub use metrics_retry_listener::{
    MetricsRetryListener, NoopRetryMetricsRecorder, RetryMetricsRecorder, RetryObservation,
};
pub use retry_metrics::{
    BlockSnapshot, NameSnapshot, RetryMetrics, RetryMetricsSnapshot, RetryOutcome,
};
pub use retry_simulation::{RetrySimulation, SleepSequence};
pub use retry_simulator::{RetrySimulator, SimState, SimulatedBackOff};
pub use retry_synchronization_manager::RetrySynchronizationManager;
pub use retry_template::RetryTemplate;
pub use retry_template_builder::RetryTemplateBuilder;
