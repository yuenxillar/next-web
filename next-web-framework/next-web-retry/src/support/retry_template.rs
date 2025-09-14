use std::{any::Any, sync::Arc, time::Duration};

use crate::{
    backoff::{
        back_off_context::BackOffContext, back_off_policy::BackOffPolicy, fixed_back_off_policy::FixedBackOffPolicy, no_back_off_policy::NoBackOffPolicy, uniform_random_back_off_policy::UniformRandomBackOffPolicy
    }, classifier::{binary_error_classifier::BinaryErrorClassifier, binary_error_classifier_builder::BinaryErrorClassifierBuilder}, error::{
        retry_error::{RetryError, WithCauseError}, AnyError
    }, policy::{
        always_retry_policy::AlwaysRetryPolicy, binary_error_classifier_retry_policy::BinaryErrorClassifierRetryPolicy, composite_retry_policy::CompositeRetryPolicy, map_retry_context_cache::MapRetryContextCache, max_attempts_retry_policy::MaxAttemptsRetryPolicy, predicate_retry_policy::PredicateRetryPolicy, retry_context_cache::RetryContextCache, simple_retry_policy::SimpleRetryPolicy, timeout_retry_policy::TimeoutRetryPolicy
    }, recovery_callback::RecoveryCallback, retry_callback::RetryCallback, retry_context::{retry_context_constants, RetryContext}, retry_listener::{DefaultRetryListener, RetryListener}, retry_operations::RetryOperations, retry_policy::RetryPolicy, retry_state::RetryState, Predicate
};
use next_web_core::{async_trait, util::any_map::AnyValue};
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Clone)]
pub struct RetryTemplate {
    back_off_policy: Arc<dyn BackOffPolicy>,
    retry_policy: Box<dyn RetryPolicy>,
    listeners: Vec<Arc<dyn RetryListener>>,
    retry_context_cache: Arc<RwLock<dyn RetryContextCache>>,
    last_error_on_exhausted: bool,
}

impl RetryTemplate {
    const GLOBAL_STATE: &str = "state.global";

    fn _default() -> Self {
        Self {
            back_off_policy: Arc::new(NoBackOffPolicy::default()),
            retry_policy: Box::new(SimpleRetryPolicy::with_max_attempts(3)),
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
        &mut self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: Option<&dyn RecoveryCallback<T>>,
        state: Option<&dyn RetryState>,
    ) -> Result<T, RetryError> 
    where T: 'static
    {

        // Allow the retry policy to initialise itself...
        let mut context = self.open(self.retry_policy.as_ref(), state).await.unwrap();

        // trace!("RetryContext retrieved: {:?}", context);

        // Make sure the context is available globally for clients who need
        // it...
        // RetrySynchronizationManager.register(context);

        let mut last_error: Option<Box<dyn AnyError>> = None;
        let mut exhausted = false;


        let block = async {
            // Give clients a chance to enhance the context...
            let running = self.do_open_interceptors(retry_callback, context.as_ref());

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
                    self.retry_policy.as_mut().get_max_attempts().into(),
                );
            }

            // Get or Start the backoff context...
            let mut back_off_context: Option<&dyn BackOffContext> = None;
            let resource = context.get_attribute("backOffContext");
            if let Some(resource) = resource {
                back_off_context = Some(resource);
            }

            if back_off_context.is_none() {
                back_off_context = self.back_off_policy.as_ref().start(context.as_ref());

                if let Some(back_off_context) = back_off_context {
                    context.set_attribute(
                        "backOffContext",
                        back_off_context
                            .get_value()
                            .map(Clone::clone)
                            .unwrap_or_default(),
                    );
                }
            }

            /*
             * We allow the whole loop to be skipped if the policy or context already
             * forbid the first try. This is used in the case of external retry to allow a
             * recovery in handleRetryExhausted without the callback processing (which
             * would throw an exception).
             */
            while self.can_retry(self.retry_policy.as_ref(), context.as_ref()) && context.is_exhausted_only() {
                // Reset the last exception, so if we are successful
                // the close interceptors will not think we failed...
                last_error = None;
                let result = retry_callback.do_with_retry(context.as_ref()).await;
                match result {
                    Ok(result) => {
                        self.do_on_success_interceptors(retry_callback, context.as_ref(), &result);
                        return Ok(result);
                    }
                    Err(error) => {
                        last_error = Some(Box::new(error.clone()));
                        
                        let cloned_context = context.clone();
                        let e = match self.register_error(self.retry_policy.as_ref() , state, cloned_context, 
                            None
                        ).await
                         {
                                Ok(_) => None,
                                Err(_) => Some(RetryError::TerminatedRetryError(
                                    WithCauseError {
                                    msg: "Retry terminated abnormally by interceptor before first attempt".to_string(),
                                    cause: None
                                    }
                                ))
                        };
                        self.do_on_error_interceptors(retry_callback, context.as_ref(), &error);

                        match e {
                            Some(error) => return Err(error),
                            None => {}
                        };

                        if self.can_retry(self.retry_policy.as_ref(), context.as_ref())
                            && !context.is_exhausted_only()
                        {
                            if let Some(bfc) = back_off_context {
                                match self.back_off_policy.as_ref().backoff(bfc) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        match e {
                                            RetryError::BackOffInterruptedError(error) => {
                                                // last_error = Some(e.clone());
                                                return Err(RetryError::BackOffInterruptedError(
                                                    error,
                                                ));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }

                        // log
                        if self.should_rethrow(self.retry_policy.as_ref(), context.as_ref(), state) {
                            return Err(RetryError::Custom("xxx".to_string()));
                        }
                    }
                };

                /*
                 * A stateful attempt that can retry may rethrow the exception before now,
                 * but if we get this far in a stateful retry there's a reason for it,
                 * like a circuit breaker or a rollback classifier.
                 */
                exhausted = true;
                return self
                    .handle_retry_exhausted(recovery_callback, context.as_mut(), state)
                    .await;
            }

            Err(RetryError::Custom("()".to_string()))
        };

        let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| block)) {
            Ok(future) => future.await,
            Err(e) => {
                let mut msg = "Unknown error".to_string();
                if let Some(error) = e.downcast_ref::<&str>() {
                    msg = error.to_string();
                }
                if let Some(error) = e.downcast_ref::<String>() {
                    msg = error.clone();
                }
                return Err(RetryError::Custom(format!("{}", msg)));
            }
        };

        // end
        self.close(
            self.retry_policy.as_ref(),
            context.as_mut(),
            state,
            last_error.is_none() || exhausted,
        )
        .await;
        self.do_close_interceptors(retry_callback, context.as_ref(), last_error.as_deref());

        result
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
                    context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));
                }
            }
            None => {
                retry_policy.close(context);
                context.set_attribute(retry_context_constants::CLOSED, AnyValue::Boolean(true));
            }
        }
    }

    pub async fn register_error(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: Option<&dyn RetryState>,
        context: impl Into<Box<dyn RetryContext>>,
        error: Option<&dyn AnyError>,
    ) -> Result<(), RetryError> {
        let mut context = context.into();
        retry_policy.register_error(context.as_mut(), error);
        self.register_context(context, state).await?;

        Ok(())
    }

    pub async fn register_context(
        &self,
        context: impl Into<Box<dyn RetryContext>>,
        state: Option<&dyn RetryState>,
    ) -> Result<(), RetryError> {
        let context = context.into();
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
                    self.retry_context_cache.write().await.put(k, context);
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
    ) -> Result<Box<dyn RetryContext>, RetryError> {
        if state.is_none() {
            return Ok(self.do_open_internal_with_retry_policy(retry_policy).await);
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
            return Ok(self.do_open_internal(retry_policy, state).await);
        }

        // If there is no cache hit we can avoid the possible expense of the
        // cache re-hydration.
        if !self.retry_context_cache.read().await.contains_key(key) {
            return Ok(self.do_open_internal(retry_policy, state).await);
        }

        match self.retry_context_cache.write().await.get_mut(key) {
            Some(context) => {
                context.remove_attribute(retry_context_constants::CLOSED);
                context.remove_attribute(retry_context_constants::EXHAUSTED);
                context.remove_attribute(retry_context_constants::RECOVERED);
                // return Ok(Box::new(context));
                // TODO
                return Err(RetryError::Custom("todo".to_string()));
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
                return Ok(self.do_open_internal(retry_policy, state).await);
            }
        }
    }

    pub async fn do_open_internal(
        &self,
        retry_policy: &dyn RetryPolicy,
        state: Option<&dyn RetryState>,
    ) -> Box<dyn RetryContext> {
        let mut context = retry_policy.open(None);
        if let Some(state) = state {
            context.set_attribute(
                retry_context_constants::STATE_KEY,
                state
                    .get_key()
                    .map(|s| AnyValue::String(s.clone()))
                    .unwrap_or(AnyValue::Null),
            );
        }

        if context.has_attribute(Self::GLOBAL_STATE) {
            self.register_context(context.clone(), state).await.unwrap();
        }

        return context;
    }

    pub async fn do_open_internal_with_retry_policy(
        &self,
        retry_policy: &dyn RetryPolicy,
    ) -> Box<dyn RetryContext> {
        self.do_open_internal(retry_policy, None).await
    }

    pub async fn handle_retry_exhausted<T>(
        &self,
        recovery_callback: Option<&dyn RecoveryCallback<T>>,
        context: &mut dyn RetryContext,
        state: Option<&dyn RetryState>,
    ) -> Result<T, RetryError> {
        context.set_attribute(retry_context_constants::EXHAUSTED, AnyValue::Boolean(true));
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
            .map(|v| {
                if v.is_boolean() {
                    v.as_boolean().unwrap_or_default()
                } else {
                    false
                }
            })
            .unwrap_or_default();
        if let Some(recovery_callback) = recovery_callback {
            if do_recover {
                let recovered = recovery_callback.recover(context);

                match recovered {
                    Ok(recovered) => {
                        context.set_attribute(
                            retry_context_constants::RECOVERED,
                            AnyValue::Boolean(true),
                        );
                        return Ok(recovered);
                    }
                    Err(_) => {
                        return Err(RetryError::Custom("UndeclaredThrowableError".to_string()));
                    }
                }
            } else {
                debug!("Retry exhausted and recovery disabled for this error")
            }
        }

        if state.is_some() {
            debug!("Retry exhausted after last attempt with no recovery path.");
            self.rethrow(
                context,
                "Retry exhausted after last attempt with no recovery path",
                self.last_error_on_exhausted || !do_recover,
            );
        }
        Err(RetryError::Default(WithCauseError {
            msg: "Exception in retry".to_string(),
            cause: context.get_last_error(),
        }))
    }

    fn rethrow(&self, context: &dyn RetryContext, msg: impl ToString, wrap: bool) -> RetryError {
        if wrap {
            return match context.get_last_error() {
                Some(error) => RetryError::Default(WithCauseError {
                    msg: "RetryError default".to_string(),
                    cause: Some(error),
                }),
                None => {
                    RetryError::Custom("Retry exhausted with no last error to rethrow".to_string())
                }
            };
        } else {
            RetryError::ExhaustedRetryError(WithCauseError {
                msg: msg.to_string(),
                cause: context.get_last_error(),
            })
        }
    }

    pub fn do_open_interceptors<T>(
        &self,
        _callback: &dyn RetryCallback<T>,
        context: &dyn RetryContext,
    ) -> bool {
        let mut result = true;

        self.listeners.iter().for_each(|listener| {
            result = result && listener.open(context);
        });

        result
    }

    pub fn do_close_interceptors<T>(
        &self,
        _callback: &dyn RetryCallback<T>,
        context: &dyn RetryContext,
        last_error: Option<&dyn AnyError>,
    ) {
        for listener in self.listeners.iter().rev() {
            listener.close(context, last_error);
        }
    }

    pub fn do_on_success_interceptors<T>(
        &self,
        _callback: &dyn RetryCallback<T>,
        context: &dyn RetryContext,
        result: &T,
    ) 
    where T: Any
    {
        for listener in self.listeners.iter().rev() {
            listener.on_success(context, result);
        }
    }

    pub fn do_on_error_interceptors<T>(
        &self,
        _callback: &dyn RetryCallback<T>,
        context: &dyn RetryContext,
        error: &dyn AnyError,
    ) {
        for listener in self.listeners.iter().rev() {
            listener.on_error(context, error);
        }
    }

    pub fn should_rethrow(
        &self,
        retry_policy: &dyn RetryPolicy,
        context: &dyn RetryContext,
        state: Option<&dyn RetryState>,
    ) -> bool {
        false
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
        self.retry_policy = Box::new(retry_policy);
    }
}

#[async_trait]
impl<T> RetryOperations<T> for RetryTemplate 
where T: Send,
T: 'static
{
    async fn execute(&mut self, retry_callback: &dyn RetryCallback<T>) -> Result<T, RetryError> {
        self.do_execute(retry_callback, None, None).await
    }

    async fn execute_with_recovery(
        &mut self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, Some(recovery_callback), None)
            .await
    }

    async fn execute_with_state(
        &mut self,
        retry_callback: &dyn RetryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, None, Some(state)).await
    }

    async fn execute_with_all(
        &mut self,
        retry_callback: &dyn RetryCallback<T>,
        recovery_callback: &dyn RecoveryCallback<T>,
        state: &dyn RetryState,
    ) -> Result<T, RetryError> {
        self.do_execute(retry_callback, Some(recovery_callback), Some(state))
            .await
    }
}

#[derive(Clone, Default)]
pub struct RetryTemplateBuilder {
    base_retry_policy: Option<Arc<dyn RetryPolicy>>,
    back_off_policy: Option<Arc<dyn BackOffPolicy>>,
    listeners: Option<Vec<Arc<dyn RetryListener>>>,
    classifier_builder: Option<BinaryErrorClassifierBuilder>,
    retry_on_predicate: Option<Arc<dyn Predicate<RetryError>>>,
}

impl RetryTemplateBuilder {
    pub fn max_attempts(mut self, max_attempts: u16) -> Self {
        assert!(max_attempts > 0, "Number of attempts should be positive");
        assert!(self.base_retry_policy.is_none(), "You have already selected another retry policy");
        self.base_retry_policy = Some(Arc::new(MaxAttemptsRetryPolicy::new(max_attempts)));
        self
    }

    pub fn with_timeout(mut self, timeout_millis: u64) -> Self {
        assert!(timeout_millis > 0, "timeoutMillis should be greater than 0");
        assert!(self.base_retry_policy.is_none(), "You have already selected another retry policy");
        self.base_retry_policy = Some(Arc::new(TimeoutRetryPolicy::new(timeout_millis)));
        self
    }

    pub fn with_timeout_from_duration(self, duration: Duration) -> Self {
        assert!(duration.as_millis() > 0, "duration should be greater than 0");
        self.with_timeout(duration.as_millis() as u64)
    }

    pub fn infinite_retry(mut self) -> Self {
        assert!(self.base_retry_policy.is_none(), "You have already selected another retry policy");
        self.base_retry_policy = Some(Arc::new(AlwaysRetryPolicy::default()));
        self
    }

    pub fn custom_policy(mut self, policy: impl RetryPolicy + 'static)  -> Self {
		assert!(self.base_retry_policy.is_none(), "You have already selected another retry policy");
		self.base_retry_policy = Some(Arc::new(policy));
        self
    }

    pub fn exponential_backoff(mut self,initial_interval:u64, multiplier: f32, max_interval: u64) -> Self {
        assert!(self.back_off_policy.is_none(), "You have already selected backoff policy");
        assert!(initial_interval >= 1, "Initial interval should be >= 1");
        assert!(multiplier > 1.0, "Multiplier should be > 1");
        assert!(max_interval > initial_interval, "Max interval should be > than initial interval");


        self
    }

    pub fn fixed_backoff(mut self, interval: u64) -> Self {
        assert!(self.back_off_policy.is_none(), "You have already selected backoff policy");
        assert!(interval >= 1, "Interval should be >= 1");
        let mut policy = FixedBackOffPolicy::new();
        policy.set_back_off_period(interval);
        self.back_off_policy = Some(Arc::new(policy));
        self
    }

    pub fn uniform_random_backoff(mut self, min_interval: u64, max_interval: u64) -> Self {
        assert!(self.back_off_policy.is_none(), "You have already selected backoff policy");
		assert!(min_interval >= 1, "Min interval should be >= 1");
		assert!(max_interval >= 1, "Max interval should be >= 1");
		assert!(max_interval > min_interval, "Max interval should be > than min interval");

        let mut policy = UniformRandomBackOffPolicy::new();
		policy.set_min_back_off_period(min_interval);
		policy.set_max_back_off_period(max_interval);
		self.back_off_policy = Some(Arc::new(policy));

        self
    }

    pub fn no_backoff(mut self) -> Self {
        assert!(self.back_off_policy.is_none(), "You have already selected backoff policy");
        self.back_off_policy = Some(Arc::new(NoBackOffPolicy::new()));
        self
    }

    pub fn custom_backoff(mut self,  back_off_policy: impl BackOffPolicy + 'static)-> Self {
        assert!(self.back_off_policy.is_none(), "You have already selected backoff policy");
		self.back_off_policy = Some(Arc::new(back_off_policy));

        self
    }

    pub fn retry_on(mut self, error: RetryError) -> Self {
        self
    }

    pub fn not_retry_on(mut self, error: RetryError) -> Self {
        self
    }

    pub fn traversing_causes(mut self) -> Self {
        self
    }

    pub fn with_listener(mut self, listner: impl RetryListener + 'static) -> Self {
        if let Some(listeners) = &mut self.listeners {
            listeners.push(Arc::new(listner));
        }else {
            self.listeners = Some(vec![Arc::new(listner)]);
        }
        self
    }

    pub fn with_listeners(mut self, listeners: Vec<impl RetryListener + 'static>) -> Self 
    {
        if let Some(self_listeners) = &mut self.listeners {
            self_listeners.extend(listeners.into_iter().map(|s| Arc::new(s) as Arc<dyn RetryListener + 'static>).collect::<Vec<_>>());
        }else {
            let s= listeners.into_iter().map(|s| Arc::new(s) as Arc<dyn RetryListener + 'static>).collect();
            self.listeners = Some(s);
        }
        self
    }

    pub fn build(mut self) -> RetryTemplate {
        let mut retry_template = RetryTemplate::default();
        
        if self.base_retry_policy.is_none() {
            self.base_retry_policy = Some(Arc::new(MaxAttemptsRetryPolicy::default()));
        }

        let mut  exception_retry_policy: Option<Arc<dyn RetryPolicy>> = None;
        if self.retry_on_predicate.is_none() {
            let esxception_classifier: BinaryErrorClassifier = match &self.classifier_builder {
                Some(classifier_builder) => classifier_builder.to_owned().build(),
                None => BinaryErrorClassifier::default_classifier()
            };
            exception_retry_policy = Some(Arc::new(BinaryErrorClassifierRetryPolicy::new(esxception_classifier)));
        }else {
            exception_retry_policy = Some(
                Arc::new(
                    PredicateRetryPolicy::new(
                        std::mem::replace(&mut self.retry_on_predicate, None).unwrap(),
                    )
                )
            );
        }

        let mut final_policy = CompositeRetryPolicy::new();


        final_policy.set_policies(vec![self.base_retry_policy.map(|v| v.clone()).unwrap(), exception_retry_policy.unwrap()]);
		retry_template.set_retry_policy(final_policy);


        // Backoff policy
        if self.back_off_policy.is_none() {
            self.back_off_policy = Some(Arc::new(NoBackOffPolicy::new()));
        }

        let back_off_policy = std::mem::replace(
            &mut self.back_off_policy,
            None
        );
        retry_template.back_off_policy = back_off_policy.unwrap();

        // Listeners
       if let Some(listeners) = self.listeners {
           retry_template.set_listeners(listeners);
       }

       retry_template
    }
}

impl Default for RetryTemplate {
    fn default() -> Self {
        RetryTemplateBuilder::default().build()
    }
}
