use std::{error::Error, sync::Arc};

use crate::{
    backoff::{back_off_policy::BackOffPolicy, no_back_off_policy::NoBackOffPolicy},
    error::{CloneableError, retry_error::RetryError},
    policy::{
        map_retry_context_cache::MapRetryContextCache, retry_context_cache::RetryContextCache,
        simple_retry_policy::SimpleRetryPolicy,
    },
    recovery_callback::RecoveryCallback,
    retry_callback::RetryCallback,
    retry_context::{RetryContext, retry_context_constants},
    retry_listener::{DefaultRetryListener, RetryListener},
    retry_operations::RetryOperations,
    retry_policy::RetryPolicy,
    retry_state::RetryState,
};
use async_trait::async_trait;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RetryTemplate {
    back_off_policy: Arc<dyn BackOffPolicy>,
    retry_policy: Arc<dyn RetryPolicy>,
    listeners: Vec<Arc<dyn RetryListener>>,
    retry_context_cache: Arc<RwLock<dyn RetryContextCache>>,
    last_error_on_exhausted: bool,
}

impl RetryTemplate {
    const GLOBAL_STATE: &str = "state.global";

    fn _default() -> Self {
        Self {
            back_off_policy: Arc::new(NoBackOffPolicy::default()),
            retry_policy: Arc::new(SimpleRetryPolicy::with_max_attempts(3)),
            listeners: vec![Arc::new(DefaultRetryListener {})],
            retry_context_cache: Arc::new(RwLock::new(MapRetryContextCache::new())),
            last_error_on_exhausted: false,
        }
    }

    pub fn builder() -> RetryTemplateBuilder {
        RetryTemplateBuilder::default()
    }

    pub fn do_execute<T, E: Error>(
        &self,
        retry_callback: &dyn RetryCallback<T, E>,
        recovery_callback: Option<&dyn RecoveryCallback<T>>,
        state: Option<&dyn RetryState>,
    ) -> Result<T, E> {
        todo!()
    }

    // =====================
    pub fn register_listener<T>(&mut self, listener: T)
    where
        T: RetryListener + 'static,
    {
        self.register_listener_with_index(listener, self.listeners.len());
    }

    pub fn register_listener_with_index<T>(&mut self, listener: T, index: usize)
    where
        T: RetryListener + 'static,
    {
        if index >= self.listeners.len() {
            self.listeners.push(Arc::new(listener));
        } else {
            self.listeners.insert(index, Arc::new(listener));
        }
    }

    pub fn has_listeners(&self) -> bool {
        self.listeners.len() > 0
    }

    pub fn can_retry(&self, retry_policy: &dyn RetryPolicy, context: &dyn RetryContext) -> bool {
        retry_policy.can_retry(context)
    }

    pub async fn close(
        &self,
        retry_policy: &dyn RetryPolicy,
        context: &mut dyn RetryContext,
        state: Option<&dyn RetryState>,
        succeeded: bool,
    ) {
        match state {
            Some(state) => {
                if succeeded {
                    if !context.has_attribute(Self::GLOBAL_STATE) {
                        self.retry_context_cache
                            .write()
                            .await
                            .remove(state.get_key().unwrap_or_default())
                    }
                    retry_policy.close(context);
                    context.set_attribute(retry_context_constants::CLOSED, true);
                }
            }
            None => {
                retry_policy.close(context);
                context.set_attribute(retry_context_constants::CLOSED, true);
            }
        }
    }

    pub fn register_error(
        &mut self,
        retry_policy: &mut dyn RetryPolicy,
        state: Option<&dyn RetryState>,
        context: &mut dyn RetryContext,
        error: Option<&dyn CloneableError>,
    ) {
        retry_policy.register_error(context, error);
        self.register_context(context, state);
    }

    pub async fn register_context(
        &mut self,
        context: &dyn RetryContext,
        state: Option<&dyn RetryState>,
    ) -> Result<(), RetryError> {
        match state {
            Some(state) => {
                let key = state.get_key();
                if let Some(k) = key {
                    if context.get_retry_count() > 1
                        && self.retry_context_cache.read().await.contains_key(k)
                    {
                        return Err(RetryError::Custom(format!(
                            "{}{}{}",
                            "Inconsistent state for failed item key: cache key has changed. ",
                            "Consider whether equals() or hashCode() for the key might be inconsistent, ",
                            "or if you need to supply a better key"
                        )));
                    }
                    self.retry_context_cache.write().await.put(k, Arc::new(context.to_owned()));
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub fn open(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: &dyn RetryState,
    ) -> impl RetryContext {
        todo!()
    }

    pub fn do_open_internal(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: &dyn RetryState,
    ) -> impl RetryContext {
        todo!()
    }

    pub fn handle_retry_exhausted<T>(
        &self,
        recovery_callback: &dyn RecoveryCallback<T>,
        context: &dyn RetryContext,
        state: &dyn RetryState,
    ) -> impl RetryContext {
    }

    pub fn do_open_interceptors<T, E>(
        &self,
        callback: &dyn RetryCallback<T, E>,
        context: &dyn RetryContext,
    ) -> bool {
        false
    }

    pub fn do_close_interceptors<T, E>(
        &self,
        callback: &dyn RetryCallback<T, E>,
        context: &dyn RetryContext,
        last_error: E,
    ) -> bool {
        false
    }

    pub fn do_on_success_interceptors<T, E>(
        &self,
        callback: &dyn RetryCallback<T, E>,
        context: &dyn RetryContext,
        result: &T,
    ) -> bool {
        false
    }

    pub fn do_on_error_interceptors<T, E>(
        &self,
        callback: &dyn RetryCallback<T, E>,
        context: &dyn RetryContext,
        error: &E,
    ) -> bool {
        false
    }

    // pub fn wrap_if_necessary<E>(error: E) -> Result<> {

    // }
}

impl RetryTemplate {
    pub fn set_last_error_on_exhausted(&mut self, last_error_on_exhausted: bool) {
        self.last_error_on_exhausted = last_error_on_exhausted;
    }

    pub fn set_retry_context_cahe<T>(&mut self, context_cahe: T)
    where
        T: RetryContextCache + 'static,
    {
        self.retry_context_cache = Arc::new(context_cahe);
    }

    pub fn set_listeners(&mut self, listeners: Vec<Arc<dyn RetryListener>>) {
        self.listeners = listeners;
    }

    pub fn set_back_off_policy<T>(&mut self, back_off_policy: T)
    where
        T: BackOffPolicy + 'static,
    {
        self.back_off_policy = Arc::new(back_off_policy);
    }

    pub fn set_retry_policy<T>(&mut self, retry_policy: T)
    where
        T: RetryPolicy + 'static,
    {
        self.retry_policy = Arc::new(retry_policy);
    }
}

#[async_trait]
impl<T, E> RetryOperations<T, E> for RetryTemplate
where
    E: Error,
{
    async fn execute(&self, retry_callback: &dyn RetryCallback<T, E>) -> Result<T, E> {
        todo!()
    }

    async fn execute_with_recovery(
        &self,
        retry_callback: &dyn RetryCallback<T, E>,
        recovery_callback: &dyn RecoveryCallback<T>,
    ) -> Result<T, E> {
        todo!()
    }

    async fn execute_with_state(
        &self,
        retry_callback: &dyn RetryCallback<T, E>,
        state: &dyn RetryState,
    ) -> Result<T, E> {
        todo!()
    }

    async fn execute_with_all(
        &self,
        retry_callback: &dyn RetryCallback<T, E>,
        recovery_callback: &dyn RecoveryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, E> {
        todo!()
    }
}

#[derive(Clone, Default)]
pub struct RetryTemplateBuilder {
    base_retry_policy: Option<Arc<dyn RetryPolicy>>,
    back_off_policy: Option<Arc<dyn BackOffPolicy>>,
    listeners: Option<Vec<Arc<dyn RetryListener>>>,
}

impl RetryTemplateBuilder {
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self
    }

    pub fn within_millis(mut self, within_millis: u64) -> Self {
        self
    }

    pub fn with_timeout(mut self, within_millis: u64) -> Self {
        self
    }

    pub fn infinite_retry(mut self) -> Self {
        self
    }

    pub fn custom_policy(mut self) -> Self {
        self
    }

    pub fn exponential_backoff(mut self) -> Self {
        self
    }

    pub fn fixed_backoff(mut self) -> Self {
        self
    }

    pub fn uniform_random_backoff(mut self) -> Self {
        self
    }

    pub fn no_backoff(mut self) -> Self {
        self
    }

    pub fn custom_backoff(mut self) -> Self {
        self
    }

    pub fn retry_on(mut self) -> Self {
        self
    }

    pub fn not_retry_on(mut self) -> Self {
        self
    }

    pub fn traversing_causes(mut self) -> Self {
        self
    }

    pub fn with_listener(mut self) -> Self {
        self
    }

    pub fn build(self) -> RetryTemplate {
        todo!()
        // RetryTemplate {}
    }
}

impl Default for RetryTemplate {
    fn default() -> Self {
        RetryTemplateBuilder::default().build()
    }
}
