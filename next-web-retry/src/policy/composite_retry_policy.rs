//! A policy that combines the answers of several policies.

use std::sync::Arc;

use next_web_core::Named;

use crate::{
    context::retry_context_support::RetryContextSupport,
    error::retry_error::RetryError,
    impl_retry_context,
    retry_context::RetryContext,
    retry_policy::{NO_MAXIMUM_ATTEMPTS_SET, RetryPolicy},
};

/// A policy that delegates to several policies and combines their answers.
///
/// In the default, pessimistic mode every policy must allow the next attempt. In optimistic
/// mode one is enough. Either way the policies used for a retry block are the ones that were
/// configured when its context was created, so reconfiguring the policy does not corrupt a
/// retry that is already in flight.
#[derive(Clone, Default)]
pub struct CompositeRetryPolicy {
    optimistic: bool,
    policies: Vec<Arc<dyn RetryPolicy>>,
}

impl CompositeRetryPolicy {
    /// Creates an empty, pessimistic composite.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the policies to delegate to.
    pub fn set_policies<V>(&mut self, policies: V)
    where
        V: IntoIterator<Item = Arc<dyn RetryPolicy>>,
    {
        self.policies = policies.into_iter().collect();
    }

    /// Sets whether one policy is enough to allow a retry.
    pub fn set_optimistic(&mut self, optimistic: bool) {
        self.optimistic = optimistic;
    }

    /// Returns whether the composite is optimistic.
    pub fn is_optimistic(&self) -> bool {
        self.optimistic
    }

    /// Returns the configured policies.
    pub fn policies(&self) -> &[Arc<dyn RetryPolicy>] {
        &self.policies
    }
}

impl RetryPolicy for CompositeRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let any: &dyn std::any::Any = context;
        let Some(context) = any.downcast_ref::<CompositeRetryContext>() else {
            return false;
        };

        if self.optimistic {
            context
                .policies
                .iter()
                .zip(context.contexts.iter())
                .any(|(policy, context)| policy.can_retry(context.as_ref()))
        } else {
            context
                .policies
                .iter()
                .zip(context.contexts.iter())
                .all(|(policy, context)| policy.can_retry(context.as_ref()))
        }
    }

    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
        let contexts = self
            .policies
            .iter()
            .map(|policy| policy.open(parent.clone()))
            .collect::<Vec<_>>();

        Arc::new(CompositeRetryContext {
            contexts,
            policies: self.policies.clone(),
            context_support: RetryContextSupport::with_parent(parent),
        })
    }

    fn close(&self, context: &dyn RetryContext) {
        let any: &dyn std::any::Any = context;
        let Some(context) = any.downcast_ref::<CompositeRetryContext>() else {
            return;
        };

        for (policy, child) in context.policies.iter().zip(context.contexts.iter()) {
            policy.close(child.as_ref());
        }
    }

    fn register_error(&self, context: &dyn RetryContext, error: &RetryError) {
        let any: &dyn std::any::Any = context;
        let Some(context) = any.downcast_ref::<CompositeRetryContext>() else {
            return;
        };

        for (policy, child) in context.policies.iter().zip(context.contexts.iter()) {
            policy.register_error(child.as_ref(), error);
        }
        context.context_support.register_error(error);
    }

    fn get_max_attempts(&self) -> u16 {
        self.policies
            .iter()
            .map(|policy| policy.get_max_attempts())
            .filter(|max_attempts| *max_attempts != NO_MAXIMUM_ATTEMPTS_SET)
            .min()
            .unwrap_or(NO_MAXIMUM_ATTEMPTS_SET)
    }
}

impl Named for CompositeRetryPolicy {
    fn name(&self) -> &str {
        "CompositeRetryPolicy"
    }
}

/// The context of a [`CompositeRetryPolicy`].
///
/// It keeps one child context per policy so each policy counts attempts independently, and
/// it is itself a retry context so listeners can inspect the composite attempt count.
#[derive(Clone)]
struct CompositeRetryContext {
    contexts: Vec<Arc<dyn RetryContext>>,
    policies: Vec<Arc<dyn RetryPolicy>>,
    context_support: RetryContextSupport,
}

impl_retry_context!(CompositeRetryContext);

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::CompositeRetryPolicy;
    use crate::{
        error::retry_error::RetryError,
        policy::{
            always_retry_policy::AlwaysRetryPolicy,
            max_attempts_retry_policy::MaxAttemptsRetryPolicy,
            never_retry_policy::NeverRetryPolicy,
        },
        retry_policy::RetryPolicy,
    };

    #[test]
    fn pessimistic_mode_requires_every_policy_to_allow_the_retry() {
        let mut composite = CompositeRetryPolicy::new();
        composite.set_policies(vec![
            Arc::new(MaxAttemptsRetryPolicy::new(3)) as Arc<dyn RetryPolicy>,
            Arc::new(NeverRetryPolicy::new()) as Arc<dyn RetryPolicy>,
        ]);
        let context = composite.open(None);

        assert!(composite.can_retry(context.as_ref()));
        composite.register_error(context.as_ref(), &RetryError::custom("failed"));
        assert!(!composite.can_retry(context.as_ref()));
        assert_eq!(context.get_retry_count(), 1);
    }

    #[test]
    fn optimistic_mode_needs_only_one_policy_to_allow_the_retry() {
        let mut composite = CompositeRetryPolicy::new();
        composite.set_optimistic(true);
        composite.set_policies(vec![
            Arc::new(AlwaysRetryPolicy::new()) as Arc<dyn RetryPolicy>,
            Arc::new(NeverRetryPolicy::new()) as Arc<dyn RetryPolicy>,
        ]);
        let context = composite.open(None);
        composite.register_error(context.as_ref(), &RetryError::custom("failed"));

        assert!(composite.can_retry(context.as_ref()));
    }

    #[test]
    fn the_lowest_configured_limit_wins() {
        let mut composite = CompositeRetryPolicy::new();
        composite.set_policies(vec![
            Arc::new(MaxAttemptsRetryPolicy::new(5)) as Arc<dyn RetryPolicy>,
            Arc::new(MaxAttemptsRetryPolicy::new(2)) as Arc<dyn RetryPolicy>,
            Arc::new(AlwaysRetryPolicy::new()) as Arc<dyn RetryPolicy>,
        ]);

        assert_eq!(composite.get_max_attempts(), 2);
    }

    #[test]
    fn a_composite_without_a_limit_reports_the_sentinel() {
        let mut composite = CompositeRetryPolicy::new();
        composite.set_policies(vec![
            Arc::new(AlwaysRetryPolicy::new()) as Arc<dyn RetryPolicy>
        ]);

        assert_eq!(composite.get_max_attempts(), 0);
    }

    #[test]
    fn child_contexts_inherit_the_parent() {
        let parent: Arc<dyn crate::retry_context::RetryContext> =
            Arc::new(crate::context::retry_context_support::RetryContextSupport::new());
        let mut composite = CompositeRetryPolicy::new();
        composite.set_policies(vec![
            Arc::new(AlwaysRetryPolicy::new()) as Arc<dyn RetryPolicy>
        ]);

        let context = composite.open(Some(parent));

        assert!(context.get_parent().is_some());
    }
}
