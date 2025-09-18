use std::{any::Any, sync::Arc};

use crate::{
    context::retry_context_support::RetryContextSupport,
    impl_retry_context,
    retry_context::{SyncAttributeAccessor, RetryContext},
    retry_policy::{NO_MAXIMUM_ATTEMPTS_SET, RetryPolicy},
};

#[derive(Clone)]
pub struct CompositeRetryPolicy {
    optimistic: bool,
    policies: Vec<Arc<dyn RetryPolicy>>,
}

impl CompositeRetryPolicy {
    pub fn new() -> Self {
        Self {
            optimistic: false,
            policies: vec![],
        }
    }

    pub fn set_policies<V>(&mut self, policies: V)
    where
        V: IntoIterator<Item = Arc<dyn RetryPolicy>>,
    {
        self.policies = policies.into_iter().collect();
    }
}

impl RetryPolicy for CompositeRetryPolicy {
    fn can_retry(&self, context: &dyn RetryContext) -> bool {

        let any: &dyn Any = context;
        if let Some(ctx) = any.downcast_ref::<CompositeRetryContext>() {
            let mut retryable = true;

            if self.optimistic {
                retryable = false;
                for (i, c) in ctx.contexts.iter().enumerate() {
                    if ctx.policies[i].can_retry(c.as_ref()) {
                        retryable = true;
                    }
                }
            }
            else {
                for (i, c) in ctx.contexts.iter().enumerate() {
                    if !ctx.policies[i].can_retry(c.as_ref()) {
                        retryable = false;
                    }
                }
            }
            return retryable;
        }

        false
    }

    fn open(&self, context: Option<&dyn RetryContext>) -> Arc<dyn RetryContext> {
        let mut contexts = vec![];
        self.policies
            .iter()
            .for_each(|x| contexts.push(x.open(context)));
        Arc::new(CompositeRetryContext {
            contexts,
            policies: self.policies.clone(),
            // TODO parent
            context_support: RetryContextSupport::default(),
        })
    }

    fn close(&self, context: &dyn RetryContext) {
        let any: &dyn Any = context;
        if let Some(ctx) = any.downcast_ref::<CompositeRetryContext>() {
            for (i,c) in ctx.contexts.iter().enumerate() {
                ctx.policies[i].close(c.as_ref());
            }
        }
    }

    fn register_error(
        &self,
        context: &dyn RetryContext,
        error: Option<&dyn crate::error::AnyError>,
    ) {
        let any: &dyn Any = context;
        if let Some(ctx) = any.downcast_ref::<CompositeRetryContext>() {
            let policies = &ctx.policies;
            for (i, c) in ctx.contexts.iter().enumerate() {
                policies[i].register_error(c.as_ref(), error);
            }
            ctx.context_support.register_error(error);
        }
    }

    fn get_max_attempts(&self) -> u16 {
        let mut max_attempts = self
            .policies
            .iter()
            .map(|p| p.get_max_attempts())
            .filter(|max_attempts| *max_attempts != NO_MAXIMUM_ATTEMPTS_SET)
            .collect::<Vec<_>>();
        max_attempts.sort();
        max_attempts
            .first()
            .map(|x| *x)
            .unwrap_or(NO_MAXIMUM_ATTEMPTS_SET)
    }
}

#[derive(Clone)]
struct CompositeRetryContext {
    contexts: Vec<Arc<dyn RetryContext>>,
    policies: Vec<Arc<dyn RetryPolicy>>,
    context_support: RetryContextSupport,
}

impl_retry_context!(CompositeRetryContext);
