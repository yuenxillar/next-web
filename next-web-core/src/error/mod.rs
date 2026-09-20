pub mod idempotency_error;
mod illegal_state_error;
pub mod invalid_parameter_error;

mod illegal_error;

pub use illegal_error::IllegalError;
pub use illegal_state_error::IllegalStateError;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
