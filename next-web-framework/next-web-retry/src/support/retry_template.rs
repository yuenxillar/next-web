use std::sync::Arc;

use crate::{
    backoff::{back_off_policy::BackOffPolicy, no_back_off_policy::NoBackOffPolicy},
    error::{
        AnyError,
        retry_error::{RetryError, WithCauseError},
    },
    policy::{
        map_retry_context_cache::MapRetryContextCache, retry_context_cache::RetryContextCache,
        simple_retry_policy::SimpleRetryPolicy,
    },
    recovery_callback::RecoveryCallback,
    retry_callback::RetryCallback,
    retry_context::{AttributeAccessor, RetryContext, retry_context_constants},
    retry_listener::{DefaultRetryListener, RetryListener},
    retry_operations::RetryOperations,
    retry_policy::RetryPolicy,
    retry_state::RetryState,
};
use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, trace};

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

    // 2.0.5
    pub async fn do_execute<T>(
        &self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: Option<&dyn RecoveryCallback<T>>,
        state: Option<&dyn RetryState>,
    ) -> Result<T, RetryError> {
        let retry_policy = &self.retry_policy;
        let back_off_policy = &self.back_off_policy;

        // Allow the retry policy to initialise itself...
        let mut context = self.open(retry_policy.as_ref(), state).await.unwrap();

        // trace!("RetryContext retrieved: {:?}", context);

        // Make sure the context is available globally for clients who need
        // it...
        // RetrySynchronizationManager.register(context);

        let mut last_error: Option<&dyn AnyError> = None;
        let mut exhausted = false;
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| async {
            // Give clients a chance to enhance the context...
            let running = self.do_open_interceptors(retry_callback, context);

            if !running {
                return Err(RetryError::TerminatedRetryError(WithCauseError {
                    msg: "Retry terminated abnormally by interceptor before first attempt"
                        .to_string(),
                    cause: None,
                }));
            }

            if !context.has_attribute(retry_context_constants::MAX_ATTEMPTS) {
                context.set_attribute(
                    retry_context_constants::MAX_ATTEMPTS,
                    retry_policy.get_max_attempts(),
                );
            }

            // Get or Start the backoff context...
            let back_off_context: Option<&dyn BackOffPolicy> = None;
            let resource = context.get_attribute("backOffContext");
            if let Some(resource) = resource {
                back_off_context = Some(resource);
            }

            if back_off_context.is_none() {
                back_off_context = Some(&back_off_policy.start(context));

                if let Some(back_off_context) = back_off_context {
                    context.set_attribute("backOffContext", back_off_context);
                }
            }

            /*
             * We allow the whole loop to be skipped if the policy or context already
             * forbid the first try. This is used in the case of external retry to allow a
             * recovery in handleRetryExhausted without the callback processing (which
             * would throw an exception).
             */

            while self.can_retry(retry_policy, context) && context.is_exhausted_only() {
                // Reset the last exception, so if we are successful
                // the close interceptors will not think we failed...
                last_error = None;
                let result = retry_callback.do_with_retry(context).await;
                match result {
                    Ok(result) => {
                        self.do_on_success_interceptors(retry_callback, context, result);
                        return Ok(result);
                    }
                    Err(error) => {
                        last_error = error;

                        let e = match self.register_error(retry_policy, state, context, error) {
                                Ok(_) => None,
                                Err(_) => Some(RetryError::TerminatedRetryError(
                                    WithCauseError {
                            msg: "Retry terminated abnormally by interceptor before first attempt".to_string(),
                            cause: None
                                }
                                ))
                        };
                        self.do_on_error_interceptors(retry_callback, context, error);

                        if self.can_retry(retry_policy, context) && !context.is_exhausted_only() {
                            match back_off_policy.backoff(back_off_context) {
                                Ok(_) => {},
                                Err(e) => {
                                    match e {
                                        RetryError::BackOffInterruptedError(error) {
                                            last_error = Some(e.clone());
                                            return Err(error)
                                        },
                                        _ => {}
                                    }
                                },
                            }
                        }

                        // log

                        if self.should_rethrow(retry_policy, context, state) {
                            return Err(RetryError::Default(()))
                        }
                    }
                };

                /*
				 * A stateful attempt that can retry may rethrow the exception before now,
				 * but if we get this far in a stateful retry there's a reason for it,
				 * like a circuit breaker or a rollback classifier.
				 */
                 exhausted = true;
                 return self.handle_retry_exhausted(recovery_callback, context, state);
            };

            // end
            self.close(retry_policy, context, state, last_error.is_none() || exhausted).await;
            self.do_close_interceptors(retry_callback, context, last_error);


            Ok(())
        })) {
            Ok(future) => {
                future.await
            }
            Err(e) => {
                let mut msg =  "Unknown error".to_string();
                if let Some(error) = e.downcast_ref::<&str>() {
                    msg = error.to_string();
                }
                if let Some(error) = e.downcast_ref::<String>() {
                    msg = error.clone();
                }
                return Err(RetryError::Custom(format!("{}", msg)))
            },
        }
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
                            .remove(state.get_key().map(|s| s.as_str()).unwrap_or_default())
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
        mut context: impl RetryContext,
        error: Option<&dyn AnyError>,
    ) -> Result<(), RetryError> {
        retry_policy.register_error(&mut context, error);
        self.register_context(context, state);

        Ok(())
    }

    pub async fn register_context(
        &mut self,
        context: impl RetryContext,
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
                    self.retry_context_cache
                        .write()
                        .await
                        .put(k, Arc::new(context));
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub async fn open(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: Option<&dyn RetryState>,
    ) -> Result<&dyn RetryContext, RetryError> {
        if state.is_none() {
            return Ok(self.do_open_internal_with_retry_policy(retry_policy));
        }

        let key = state
            .as_ref()
            .map(|s| s.get_key().as_ref().map(|s| s.as_str()).unwrap_or_default())
            .unwrap_or_default();

        if state
            .as_ref()
            .map(|s| s.is_force_refresh())
            .unwrap_or(false)
        {
            return Ok(self.do_open_internal(retry_policy, state));
        }

        // If there is no cache hit we can avoid the possible expense of the
        // cache re-hydration.
        if !self.retry_context_cache.read().await.contains_key(key) {
            return Ok(self.do_open_internal(retry_policy, state));
        }

        match self.retry_context_cache.write().await.get_mut(key) {
            Some(context) => {
                context.remove_attribute(retry_context_constants::CLOSED);
                context.remove_attribute(retry_context_constants::EXHAUSTED);
                context.remove_attribute(retry_context_constants::RECOVERED);
                return Ok(context);
            }
            None => {
                if self.retry_context_cache.read().await.contains_key(key) {
                    return Err(RetryError::Custom(format!(
                        "{}{}{}",
                        "Inconsistent state for failed item: no history found. ",
                        "Consider whether equals() or hashCode() for the item might be inconsistent, ",
                        "or if you need to supply a better ItemKeyGenerator"
                    )));
                }
                return Ok(self.do_open_internal(retry_policy, state));
            }
        }
    }

    pub fn do_open_internal(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: Option<&dyn RetryState>,
    ) -> &dyn RetryContext {
        let context = retry_policy.open(context);
        if let Some(state) = state {
            context.set_attribute(
                retry_context_constants::STATE_KEY,
                state.get_key().map(|s| s.as_str()).unwrap_or_default(),
            );
        }

        if context.has_attribute(Self::GLOBAL_STATE) {
            self.register_context(context, state);
        }

        return context;
    }

    pub fn do_open_internal_with_retry_policy(
        &self,
        retry_policy: &dyn RetryPolicy,
    ) -> &dyn  RetryContext {
        self.do_open_internal(retry_policy, None)
    }

    pub async fn handle_retry_exhausted<T>(
        &self,
        recovery_callback: Option<&dyn RecoveryCallback<T>>,
        context: &mut dyn RetryContext,
        state: Option<&dyn RetryState>,
    ) -> Result<T, RetryError>
    where
        T:,
    {
        context.set_attribute(retry_context_constants::EXHAUSTED, true);
        if state.is_some() && !context.has_attribute(Self::GLOBAL_STATE) {
            self.retry_context_cache.write().await.remove(
                state
                    .as_ref()
                    .map(|v| v.get_key().as_ref().map(|s| s.as_str()).unwrap_or_default())
                    .unwrap_or_default(),
            );
        }

        let do_recover = context
            .get_attribute(retry_context_constants::NO_RECOVERY)
            .map(|v| *v)
            .unwrap_or_default();
        if let Some(recovery_callback) = recovery_callback {
            if do_recover {
                let recovered = recovery_callback.recover(context);

                match recovered {
                    Ok(recovered) => {
                        context.set_attribute(retry_context_constants::RECOVERED, true);
                        return Ok(recovered);
                    }
                    Err(_) => return Err(RetryError::Custom("UndeclaredThrowableError".to_string())),
                }
            } else {
                debug!("Retry exhausted and recovery disabled for this error")
            }
        }

        if  state.is_some() {
            debug!("Retry exhausted after last attempt with no recovery path.");
            self.rethrow(
                context,
                "Retry exhausted after last attempt with no recovery path",
                self.last_error_on_exhausted || !do_recover,
            );
        }
        Err(RetryError::Default( WithCauseError{
            msg: "Exception in retry".to_string(),
            cause: context.get_last_error(),
        }))
    }

    fn rethrow(
        &self,
        context: &dyn RetryContext,
        msg: impl ToString,
        wrap: bool,
    ) -> RetryError {
        if wrap {
            
            return match context.get_last_error() {
                Some(error) => 
                    RetryError::Default(
                    WithCauseError { msg: "RetryError default".to_string(), cause:  Some(error) }
                )
                ,
                None => RetryError::Custom("Retry exhausted with no last error to rethrow".to_string())
            };
        
        } else {
            RetryError::ExhaustedRetryError(
                WithCauseError {
                msg: msg.to_string(),
                cause: context.get_last_error(),
            }
            )
        }
    }

    pub fn do_open_interceptors<T>(
        &self,
        callback: &dyn RetryCallback<T>,
        context: &dyn RetryContext,
    ) -> bool {
        let mut result = true;

        self.listeners.iter().for_each(|listener| {
            result = result && listener.open(context, callback);
        });

        result
    }

    pub fn do_close_interceptors<T>(
        &self,
        callback: &dyn RetryCallback<T>,
        context: &dyn RetryContext,
        last_error: &dyn AnyError,
    ) {
        for listener in self.listeners.iter().rev() {
            listener.close(context, callback, last_error);
        }
    }

    pub fn do_on_success_interceptors<T>(
        &self,
        callback: &dyn RetryCallback<T>,
        context: &dyn RetryContext,
        result: &T,
    ) {
        for listener in self.listeners.iter().rev() {
            listener.on_success(context, callback, result);
        }
    }

    pub fn do_on_error_interceptors<T>(
        &self,
        callback: &dyn RetryCallback<T>,
        context: &dyn RetryContext,
        error: &dyn AnyError,
    ) {
        for listener in self.listeners.iter().rev() {
            listener.on_error(context, callback, error);
        }
    }
}

impl RetryTemplate {
    pub fn set_last_error_on_exhausted(&mut self, last_error_on_exhausted: bool) {
        self.last_error_on_exhausted = last_error_on_exhausted;
    }

    pub fn set_retry_context_cahe<T>(&mut self, context_cahe: T)
    where
        T: RetryContextCache + 'static,
    {
        self.retry_context_cache = Arc::new(RwLock::new(context_cahe));
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
impl<T> RetryOperations<T> for RetryTemplate
{
    async fn execute(&self, retry_callback: &dyn RetryCallback<T>) -> Result<T, RetryError> {
        self.do_execute(retry_callback, None, None).await
    }

    async fn execute_with_recovery(
        &self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, Some(recovery_callback), None).await
    }

    async fn execute_with_state(
        &self,
        retry_callback: &dyn RetryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, None, Some(state)).await
    }

    async fn execute_with_all(
        &self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, Some(recovery_callback), Some(state)).await
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
