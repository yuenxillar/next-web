pub mod illegal_state_error;
pub mod idempotency_error;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
