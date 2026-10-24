//! A policy that applies a different policy per error.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use dashmap::DashMap;
use next_web_core::Named;

use crate::{
    classifier::classifier::Classifier, context::retry_context_support::RetryContextSupport,
    error::retry_error::RetryError, impl_retry_context, retry_context::RetryContext,
    retry_policy::RetryPolicy,
};

use super::never_retry_policy::NeverRetryPolicy;

/// A policy that dynamically applies one of several policies, chosen by the last error.
///
/// The choice is made when an error is registered, not when the block starts, so a retry
/// that fails with different errors can switch between policies. Every policy that was
/// used gets its own context and is closed when the block closes.
pub struct ExceptionClassifierRetryPolicy {
    exception_classifier: Arc<dyn Classifier<RetryError, Arc<dyn RetryPolicy>>>,
}

impl ExceptionClassifierRetryPolicy {
    /// Creates a policy from an error-to-policy classifier.
    pub fn new(
        exception_classifier: Arc<dyn Classifier<RetryError, Arc<dyn RetryPolicy>>>,
    ) -> Self {
        Self {
            exception_classifier,
        }
    }

    /// Creates a policy from an explicit error-to-policy map.
    ///
    /// Errors that are not in the map are handled by [`NeverRetryPolicy`], so an
    /// unclassified failure is not retried.
    pub fn with_policy_map(policy_map: HashMap<RetryError, Arc<dyn RetryPolicy>>) -> Self {
        Self::new(Arc::new(
            crate::classifier::map_classifier::MapClassifier::new(
                policy_map,
                Arc::new(NeverRetryPolicy::new()) as Arc<dyn RetryPolicy>,
            ),
        ))
    }

    /// Replaces the classifier.
    pub fn set_exception_classifier(
        &mut self,
        exception_classifier: Arc<dyn Classifier<RetryError, Arc<dyn RetryPolicy>>>,
    ) {
        self.exception_classifier = exception_classifier;
    }
}

impl Clone for ExceptionClassifierRetryPolicy {
    fn clone(&self) -> Self {
        Self {
            exception_classifier: self.exception_classifier.clone(),
        }
    }
}

impl RetryPolicy for ExceptionClassifierRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {
        let any: &dyn std::any::Any = context;
        any.downcast_ref::<ExceptionClassifierRetryContext>()
            .map(ExceptionClassifierRetryContext::can_retry_activated)
            .unwrap_or(false)
    }

    fn open(&self, parent: Option<Arc<dyn RetryContext>>) -> Arc<dyn RetryContext> {
        Arc::new(ExceptionClassifierRetryContext {
            context_support: RetryContextSupport::with_parent(parent),
            exception_classifier: self.exception_classifier.clone(),
            activated: Mutex::new(None),
            contexts: DashMap::new(),
        })
    }

    fn close(&self, context: &dyn RetryContext) {
        let any: &dyn std::any::Any = context;
        if let Some(context) = any.downcast_ref::<ExceptionClassifierRetryContext>() {
            context.close_activated();
        }
    }

    fn register_error(&self, context: &dyn RetryContext, error: &RetryError) {
        let any: &dyn std::any::Any = context;
        if let Some(context) = any.downcast_ref::<ExceptionClassifierRetryContext>() {
            context.register_activated(error);
            context.context_support.register_error(error);
        }
    }
}

impl Named for ExceptionClassifierRetryPolicy {
    fn name(&self) -> &str {
        "ExceptionClassifierRetryPolicy"
    }
}

/// The policy and context that are currently active for a retry block.
struct Activated {
    policy: Arc<dyn RetryPolicy>,
    context: Arc<dyn RetryContext>,
}

/// The child contexts selected so far, keyed by the identity of their policy.
type PolicyContexts = DashMap<usize, (Arc<dyn RetryPolicy>, Arc<dyn RetryContext>)>;

/// The context of an [`ExceptionClassifierRetryPolicy`].
///
/// It keeps the child context of every policy that has been selected so far, keyed by the
/// identity of that policy.
struct ExceptionClassifierRetryContext {
    context_support: RetryContextSupport,
    exception_classifier: Arc<dyn Classifier<RetryError, Arc<dyn RetryPolicy>>>,
    activated: Mutex<Option<Activated>>,
    contexts: PolicyContexts,
}

impl ExceptionClassifierRetryContext {
    fn can_retry_activated(&self) -> bool {
        // The active policy is copied out before it is called: a policy is foreign code and
        // must never run while this context's lock is held.
        let activated = match self.activated.lock() {
            Ok(guard) => guard.as_ref().map(|activated| {
                (
                    Arc::clone(&activated.policy),
                    Arc::clone(&activated.context),
                )
            }),
            Err(_) => return false,
        };

        match activated {
            Some((policy, context)) => policy.can_retry(context.as_ref()),
            None => true,
        }
    }

    fn register_activated(&self, error: &RetryError) {
        let policy = self.exception_classifier.classify(Some(error));
        let context = self.context_for(policy.clone());

        if let Ok(mut activated) = self.activated.lock() {
            *activated = Some(Activated {
                policy: policy.clone(),
                context: context.clone(),
            });
        }

        policy.register_error(context.as_ref(), error);
    }

    fn context_for(&self, policy: Arc<dyn RetryPolicy>) -> Arc<dyn RetryContext> {
        let key = policy_key(&policy);
        if let Some(entry) = self.contexts.get(&key) {
            return entry.1.clone();
        }
        let parent = self.context_support.get_parent_shared();
        let context = policy.open(parent);
        self.contexts.insert(key, (policy, context.clone()));
        context
    }

    fn close_activated(&self) {
        // Collect first so no shard of the map is locked while a policy is closed.
        let children = self
            .contexts
            .iter()
            .map(|entry| (Arc::clone(&entry.value().0), Arc::clone(&entry.value().1)))
            .collect::<Vec<_>>();

        for (policy, context) in children {
            policy.close(context.as_ref());
        }
    }
}

impl_retry_context!(ExceptionClassifierRetryContext);

fn policy_key(policy: &Arc<dyn RetryPolicy>) -> usize {
    Arc::as_ptr(policy) as *const () as usize
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use super::ExceptionClassifierRetryPolicy;
    use crate::{
        error::retry_error::RetryError,
        policy::{always_retry_policy::AlwaysRetryPolicy, never_retry_policy::NeverRetryPolicy},
        retry_policy::RetryPolicy,
    };

    #[test]
    fn the_policy_is_chosen_by_the_error() {
        let policy = ExceptionClassifierRetryPolicy::with_policy_map(HashMap::from([
            (
                RetryError::custom("retry"),
                Arc::new(AlwaysRetryPolicy::new()) as Arc<dyn RetryPolicy>,
            ),
            (
                RetryError::custom("stop"),
                Arc::new(NeverRetryPolicy::new()) as Arc<dyn RetryPolicy>,
            ),
        ]));
        let context = policy.open(None);

        assert!(policy.can_retry(context.as_ref()));
        policy.register_error(context.as_ref(), &RetryError::custom("retry"));
        assert!(policy.can_retry(context.as_ref()));

        policy.register_error(context.as_ref(), &RetryError::custom("stop"));
        assert!(!policy.can_retry(context.as_ref()));
    }

    #[test]
    fn an_unclassified_error_is_not_retried() {
        let policy = ExceptionClassifierRetryPolicy::with_policy_map(HashMap::new());
        let context = policy.open(None);
        policy.register_error(context.as_ref(), &RetryError::custom("unknown"));

        assert!(!policy.can_retry(context.as_ref()));
        assert_eq!(context.get_retry_count(), 1);
    }
}
