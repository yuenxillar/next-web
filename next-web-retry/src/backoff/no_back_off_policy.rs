//! A back off that never waits.

use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    backoff::{
        back_off_context::BackOffContext,
        back_off_policy::BackOffPolicy,
        stateless_back_off_policy::{StatelessBackOffPolicy, stateless_backoff, stateless_start},
    },
    error::retry_error::RetryError,
    retry_context::RetryContext,
};

/// A policy that never pauses between attempts.
///
/// It is the default policy of a retry template: attempts follow each other immediately
/// unless a back off is configured explicitly.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoBackOffPolicy;

impl NoBackOffPolicy {
    /// Creates the policy.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BackOffPolicy for NoBackOffPolicy {
    async fn start(&self, context: &dyn RetryContext) -> Option<Arc<dyn BackOffContext>> {
        stateless_start(context).await
    }

    async fn backoff(&self, _context: Option<&dyn BackOffContext>) -> Result<(), RetryError> {
        stateless_backoff(self).await
    }
}

#[async_trait]
impl StatelessBackOffPolicy for NoBackOffPolicy {
    async fn do_backoff(&self) -> Result<(), RetryError> {
        Ok(())
    }
}

impl std::fmt::Display for NoBackOffPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NoBackOffPolicy []")
    }
}

#[cfg(test)]
mod tests {
    use super::NoBackOffPolicy;
    use crate::{
        backoff::back_off_policy::BackOffPolicy,
        context::retry_context_support::RetryContextSupport,
    };

    #[tokio::test]
    async fn the_policy_returns_immediately_without_state() {
        let policy = NoBackOffPolicy::new();
        let context = RetryContextSupport::new();

        assert!(policy.start(&context).await.is_none());
        policy
            .backoff(None)
            .await
            .expect("the no-op policy cannot fail");
    }
}
