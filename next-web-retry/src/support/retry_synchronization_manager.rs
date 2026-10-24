//! The task-local access point to the retry block in progress.

use std::{future::Future, sync::Arc};

use crate::retry_context::RetryContext;

tokio::task_local! {
    /// The retry context of the block the current task is running inside.
    static CURRENT_CONTEXT: Arc<dyn RetryContext>;
}

/// Gives code access to the retry block it runs inside.
///
/// A callback always receives its context, so this is only needed by code that cannot be
/// passed one: a nested retry needs the enclosing context as its parent, and a listener
/// deep in a call chain may want to see the retry that is in progress.
///
/// The original implementation kept the context in a thread local, which cannot follow a
/// task across an await point. This manager keeps it in a task local instead: the context is
/// scoped to the future passed to [`RetrySynchronizationManager::scope`], survives every await
/// inside it, and is restored
/// to the enclosing context when the future finishes, which is exactly what nested retry
/// blocks need.
pub struct RetrySynchronizationManager;

impl RetrySynchronizationManager {
    /// Returns the context of the retry block the current task runs inside, if any.
    pub fn get_context() -> Option<Arc<dyn RetryContext>> {
        CURRENT_CONTEXT.try_with(|context| context.clone()).ok()
    }

    /// Runs `future` with `context` installed as the current retry context.
    ///
    /// Nested calls restore the enclosing context when the inner future finishes.
    pub async fn scope<F>(context: Arc<dyn RetryContext>, future: F) -> F::Output
    where
        F: Future,
    {
        CURRENT_CONTEXT.scope(context, future).await
    }
}

impl std::fmt::Debug for RetrySynchronizationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetrySynchronizationManager")
            .field(
                "has_context",
                &RetrySynchronizationManager::get_context().is_some(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::RetrySynchronizationManager;
    use crate::{context::retry_context_support::RetryContextSupport, retry_context::RetryContext};

    #[tokio::test]
    async fn there_is_no_context_outside_a_scope() {
        assert!(RetrySynchronizationManager::get_context().is_none());
    }

    #[tokio::test]
    async fn the_context_survives_an_await_point() {
        let context: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::new());

        let seen = RetrySynchronizationManager::scope(context.clone(), async {
            tokio::task::yield_now().await;
            RetrySynchronizationManager::get_context()
        })
        .await;

        assert!(seen.is_some_and(|seen| Arc::ptr_eq(&seen, &context)));
    }

    #[tokio::test]
    async fn a_nested_scope_restores_the_enclosing_context() {
        let outer: Arc<dyn RetryContext> = Arc::new(RetryContextSupport::with_parent(None));
        let inner: Arc<dyn RetryContext> =
            Arc::new(RetryContextSupport::with_parent(Some(outer.clone())));

        RetrySynchronizationManager::scope(outer.clone(), async move {
            let outer_before = RetrySynchronizationManager::get_context();

            RetrySynchronizationManager::scope(inner, async {}).await;

            let outer_after = RetrySynchronizationManager::get_context();

            assert!(outer_before.is_some_and(|context| Arc::ptr_eq(&context, &outer)));
            assert!(outer_after.is_some_and(|context| Arc::ptr_eq(&context, &outer)));
        })
        .await;
    }
}
