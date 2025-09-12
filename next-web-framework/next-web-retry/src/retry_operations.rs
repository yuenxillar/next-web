use std::error::Error;

use async_trait::async_trait;

use crate::{
    error::retry_error::RetryError, recovery_callback::RecoveryCallback, retry_callback::RetryCallback, retry_state::RetryState
};

#[async_trait]
pub trait RetryOperations<T>
where
    Self: Send + Sync,
{
    async fn execute(&self, retry_callback: &dyn RetryCallback<T>) -> Result<T, RetryError>;

    async fn execute_with_recovery(
        &self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
    ) -> Result<T, RetryError>;

    async fn execute_with_state(
        &self,
        retry_callback: &dyn RetryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError>;

    async fn execute_with_all(
        &self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError>;
}
