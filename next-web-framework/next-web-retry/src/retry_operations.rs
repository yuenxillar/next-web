use next_web_core::async_trait;

use crate::{
    error::retry_error::RetryError, recovery_callback::RecoveryCallback, retry_callback::RetryCallback, retry_state::RetryState
};

#[async_trait]
pub trait RetryOperations<T>
where
    Self: Send + Sync,
{
    async fn execute(&mut self, retry_callback: &dyn RetryCallback<T>) -> Result<T, RetryError>;

    async fn execute_with_recovery(
        &mut self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
    ) -> Result<T, RetryError>;

    async fn execute_with_state(
        &mut self,
        retry_callback: &dyn RetryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError>;

    async fn execute_with_all(
        &mut self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError>;
}
