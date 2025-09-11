use std::error::Error;

use async_trait::async_trait;

use crate::{
    recovery_callback::RecoveryCallback, retry_callback::RetryCallback, retry_state::RetryState,
};

#[async_trait]
pub trait RetryOperations<T, E>
where
    Self: Send + Sync,
    E: Error,
{
    async fn execute(&self, retry_callback: &dyn RetryCallback<T, E>) -> Result<T, E>;

    async fn execute_with_recovery(
        &self,
        retry_callback: &dyn RetryCallback<T, E>,
        recovery_callback: &dyn RecoveryCallback<T>,
    ) -> Result<T, E>;

    async fn execute_with_state(
        &self,
        retry_callback: &dyn RetryCallback<T, E>,
        state: &dyn RetryState,
    ) -> Result<T, E>;

    async fn execute_with_all(
        &self,
        retry_callback: &dyn RetryCallback<T, E>,
        recovery_callback: &dyn RecoveryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, E>;
}
