pub mod backoff;
pub mod classifier;
pub mod context;
pub mod error;
pub mod policy;
pub mod recovery_callback;
pub mod retry_callback;
pub mod retry_context;
pub mod retry_listener;
pub mod retry_operations;
pub mod retry_policy;
pub mod retry_state;
pub mod support;

pub trait Predicate<T>
where
    Self: Send + Sync,
{
}
