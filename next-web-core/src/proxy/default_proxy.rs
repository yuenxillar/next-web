use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Hook trait used by [`DefaultProxy`] to intercept method execution.
///
/// Rust does not offer Java-style runtime dynamic proxies for arbitrary methods.
/// The idiomatic approach is to wrap the target object and route calls through
/// closures so proxy hooks can run before and after each invocation.
pub trait ProxyHandler<T> {
    /// Called before the target invocation starts.
    fn before(&self, _target: &T, _method: &str, _call_id: u64) {}

    /// Called after a successful invocation.
    fn after<R>(&self, _target: &T, _method: &str, _call_id: u64, _result: &R) {}

    /// Called after a failed invocation.
    fn on_error<E>(&self, _target: &T, _method: &str, _call_id: u64, _error: &E) {}
}

/// A no-op proxy handler used by default.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProxyHandler;

impl<T> ProxyHandler<T> for NoopProxyHandler {}

/// A composable handler that chains two handlers together.
///
/// Execution order follows common AOP semantics:
/// - `before`: first -> second
/// - `after`: second -> first
/// - `on_error`: second -> first
#[derive(Debug, Clone)]
pub struct CompositeProxyHandler<H1, H2> {
    first: H1,
    second: H2,
}

impl<H1, H2> CompositeProxyHandler<H1, H2> {
    /// Creates a new composite handler.
    pub fn new(first: H1, second: H2) -> Self {
        Self { first, second }
    }

    /// Returns the first handler.
    pub fn first(&self) -> &H1 {
        &self.first
    }

    /// Returns the second handler.
    pub fn second(&self) -> &H2 {
        &self.second
    }
}

impl<T, H1, H2> ProxyHandler<T> for CompositeProxyHandler<H1, H2>
where
    H1: ProxyHandler<T>,
    H2: ProxyHandler<T>,
{
    fn before(&self, target: &T, method: &str, call_id: u64) {
        self.first.before(target, method, call_id);
        self.second.before(target, method, call_id);
    }

    fn after<R>(&self, target: &T, method: &str, call_id: u64, result: &R) {
        self.second.after(target, method, call_id, result);
        self.first.after(target, method, call_id, result);
    }

    fn on_error<E>(&self, target: &T, method: &str, call_id: u64, error: &E) {
        self.second.on_error(target, method, call_id, error);
        self.first.on_error(target, method, call_id, error);
    }
}

/// A lightweight function-based handler.
///
/// This is useful when a full dedicated handler type would be overkill.
#[derive(Debug, Clone)]
pub struct FnProxyHandler<BF, AF, EF> {
    before: BF,
    after: AF,
    on_error: EF,
}

impl<BF, AF, EF> FnProxyHandler<BF, AF, EF> {
    /// Creates a new function-based proxy handler.
    pub fn new(before: BF, after: AF, on_error: EF) -> Self {
        Self {
            before,
            after,
            on_error,
        }
    }
}

impl<T, BF, AF, EF> ProxyHandler<T> for FnProxyHandler<BF, AF, EF>
where
    BF: Fn(&T, &str, u64),
    AF: Fn(&T, &str, u64),
    EF: Fn(&T, &str, u64),
{
    fn before(&self, target: &T, method: &str, call_id: u64) {
        (self.before)(target, method, call_id);
    }

    fn after<R>(&self, target: &T, method: &str, call_id: u64, _result: &R) {
        (self.after)(target, method, call_id);
    }

    fn on_error<E>(&self, target: &T, method: &str, call_id: u64, _error: &E) {
        (self.on_error)(target, method, call_id);
    }
}

/// A tracing-based handler for method entry, success and failure logging.
#[derive(Debug, Clone)]
pub struct TracingProxyHandler {
    component: String,
}

impl TracingProxyHandler {
    /// Creates a new tracing handler.
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
        }
    }
}

impl<T> ProxyHandler<T> for TracingProxyHandler {
    fn before(&self, _target: &T, method: &str, call_id: u64) {
        tracing::debug!(
            component = %self.component,
            method = method,
            call_id = call_id,
            "proxy call started"
        );
    }

    fn after<R>(&self, _target: &T, method: &str, call_id: u64, _result: &R) {
        tracing::debug!(
            component = %self.component,
            method = method,
            call_id = call_id,
            "proxy call completed"
        );
    }

    fn on_error<E>(&self, _target: &T, method: &str, call_id: u64, _error: &E) {
        tracing::warn!(
            component = %self.component,
            method = method,
            call_id = call_id,
            "proxy call failed"
        );
    }
}

/// A timing-based handler that measures elapsed time for each invocation.
#[derive(Debug, Default)]
pub struct TimingProxyHandler {
    component: String,
    starts: Mutex<HashMap<u64, Instant>>,
}

impl TimingProxyHandler {
    /// Creates a new timing handler.
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            starts: Mutex::new(HashMap::new()),
        }
    }

    fn take_elapsed(&self, call_id: u64) -> Option<std::time::Duration> {
        self.starts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&call_id)
            .map(|started_at| started_at.elapsed())
    }
}

impl<T> ProxyHandler<T> for TimingProxyHandler {
    fn before(&self, _target: &T, method: &str, call_id: u64) {
        self.starts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(call_id, Instant::now());
        tracing::trace!(
            component = %self.component,
            method = method,
            call_id = call_id,
            "timing started"
        );
    }

    fn after<R>(&self, _target: &T, method: &str, call_id: u64, _result: &R) {
        if let Some(elapsed) = self.take_elapsed(call_id) {
            tracing::debug!(
                component = %self.component,
                method = method,
                call_id = call_id,
                elapsed_ms = elapsed.as_millis() as u64,
                "proxy call completed"
            );
        }
    }

    fn on_error<E>(&self, _target: &T, method: &str, call_id: u64, _error: &E) {
        if let Some(elapsed) = self.take_elapsed(call_id) {
            tracing::warn!(
                component = %self.component,
                method = method,
                call_id = call_id,
                elapsed_ms = elapsed.as_millis() as u64,
                "proxy call failed"
            );
        }
    }
}

/// A reusable proxy wrapper for sync and async method interception.
///
/// The proxy itself does not try to automatically forward every target method.
/// Instead, callers explicitly route operations through `call` / `invoke` so the
/// hook pipeline is preserved.
#[derive(Debug)]
pub struct DefaultProxy<T, H = NoopProxyHandler> {
    target: T,
    handler: H,
    call_seq: AtomicU64,
}

impl<T> DefaultProxy<T, NoopProxyHandler> {
    /// Creates a new proxy with a no-op handler.
    pub fn new(target: T) -> Self {
        Self {
            target,
            handler: NoopProxyHandler,
            call_seq: AtomicU64::new(1),
        }
    }
}

impl<T, H> DefaultProxy<T, H> {
    /// Creates a new proxy with the provided handler.
    pub fn with_handler(target: T, handler: H) -> Self {
        Self {
            target,
            handler,
            call_seq: AtomicU64::new(1),
        }
    }

    /// Returns a shared reference to the wrapped target.
    ///
    /// Direct access bypasses proxy hooks.
    pub fn target(&self) -> &T {
        &self.target
    }

    /// Returns a mutable reference to the wrapped target.
    ///
    /// Direct access bypasses proxy hooks.
    pub fn target_mut(&mut self) -> &mut T {
        &mut self.target
    }

    /// Returns a shared reference to the configured handler.
    pub fn handler(&self) -> &H {
        &self.handler
    }

    /// Returns a mutable reference to the configured handler.
    pub fn handler_mut(&mut self) -> &mut H {
        &mut self.handler
    }

    /// Consumes the proxy and returns the wrapped target.
    pub fn into_inner(self) -> T {
        self.target
    }

    /// Consumes the proxy and returns the handler.
    pub fn into_handler(self) -> H {
        self.handler
    }

    /// Replaces the handler while keeping the target.
    pub fn with_replaced_handler<H2>(self, handler: H2) -> DefaultProxy<T, H2> {
        DefaultProxy {
            target: self.target,
            handler,
            call_seq: AtomicU64::new(self.call_seq.load(Ordering::Relaxed)),
        }
    }

    fn next_call_id(&self) -> u64 {
        self.call_seq.fetch_add(1, Ordering::Relaxed)
    }
}

impl<T, H> Clone for DefaultProxy<T, H>
where
    T: Clone,
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            handler: self.handler.clone(),
            call_seq: AtomicU64::new(self.call_seq.load(Ordering::Relaxed)),
        }
    }
}

impl<T, H> DefaultProxy<T, H>
where
    H: ProxyHandler<T>,
{
    /// Executes a non-fallible shared call through the proxy pipeline.
    pub fn call<R, F>(&self, method: &str, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let call_id = self.next_call_id();
        self.handler.before(&self.target, method, call_id);
        let result = f(&self.target);
        self.handler.after(&self.target, method, call_id, &result);
        result
    }

    /// Executes a fallible shared call through the proxy pipeline.
    pub fn invoke<R, E, F>(&self, method: &str, f: F) -> Result<R, E>
    where
        F: FnOnce(&T) -> Result<R, E>,
    {
        let call_id = self.next_call_id();
        self.handler.before(&self.target, method, call_id);
        let result = f(&self.target);
        match &result {
            Ok(value) => self.handler.after(&self.target, method, call_id, value),
            Err(error) => self.handler.on_error(&self.target, method, call_id, error),
        }
        result
    }

    /// Executes a non-fallible mutable call through the proxy pipeline.
    pub fn call_mut<R, F>(&mut self, method: &str, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let call_id = self.next_call_id();
        self.handler.before(&self.target, method, call_id);
        let result = f(&mut self.target);
        self.handler.after(&self.target, method, call_id, &result);
        result
    }

    /// Executes a fallible mutable call through the proxy pipeline.
    pub fn invoke_mut<R, E, F>(&mut self, method: &str, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut T) -> Result<R, E>,
    {
        let call_id = self.next_call_id();
        self.handler.before(&self.target, method, call_id);
        let result = f(&mut self.target);
        match &result {
            Ok(value) => self.handler.after(&self.target, method, call_id, value),
            Err(error) => self.handler.on_error(&self.target, method, call_id, error),
        }
        result
    }

    /// Executes a non-fallible async shared call through the proxy pipeline.
    pub async fn call_async<R, Fut, F>(&self, method: &str, f: F) -> R
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = R>,
    {
        let call_id = self.next_call_id();
        self.handler.before(&self.target, method, call_id);
        let result = f(&self.target).await;
        self.handler.after(&self.target, method, call_id, &result);
        result
    }

    /// Executes a fallible async shared call through the proxy pipeline.
    pub async fn invoke_async<R, E, Fut, F>(&self, method: &str, f: F) -> Result<R, E>
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = Result<R, E>>,
    {
        let call_id = self.next_call_id();
        self.handler.before(&self.target, method, call_id);
        let result = f(&self.target).await;
        match &result {
            Ok(value) => self.handler.after(&self.target, method, call_id, value),
            Err(error) => self.handler.on_error(&self.target, method, call_id, error),
        }
        result
    }

    /// Executes a non-fallible async mutable call through the proxy pipeline.
    pub async fn call_async_mut<R, Fut, F>(&mut self, method: &str, f: F) -> R
    where
        F: FnOnce(&mut T) -> Fut,
        Fut: Future<Output = R>,
    {
        let call_id = self.next_call_id();
        self.handler.before(&self.target, method, call_id);
        let result = f(&mut self.target).await;
        self.handler.after(&self.target, method, call_id, &result);
        result
    }

    /// Executes a fallible async mutable call through the proxy pipeline.
    pub async fn invoke_async_mut<R, E, Fut, F>(&mut self, method: &str, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut T) -> Fut,
        Fut: Future<Output = Result<R, E>>,
    {
        let call_id = self.next_call_id();
        self.handler.before(&self.target, method, call_id);
        let result = f(&mut self.target).await;
        match &result {
            Ok(value) => self.handler.after(&self.target, method, call_id, value),
            Err(error) => self.handler.on_error(&self.target, method, call_id, error),
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Debug, Default, Clone)]
    struct Counter {
        value: i32,
    }

    trait Calculator {
        fn add(&self, left: i32, right: i32) -> i32;
        fn divide(&self, left: i32, right: i32) -> Result<i32, &'static str>;
    }

    impl Calculator for Counter {
        fn add(&self, left: i32, right: i32) -> i32 {
            left + right + self.value
        }

        fn divide(&self, left: i32, right: i32) -> Result<i32, &'static str> {
            if right == 0 {
                Err("division by zero")
            } else {
                Ok(left / right)
            }
        }
    }

    #[derive(Debug, Clone)]
    struct CalculatorProxy<H> {
        inner: DefaultProxy<Counter, H>,
    }

    impl<H> CalculatorProxy<H> {
        fn new(inner: DefaultProxy<Counter, H>) -> Self {
            Self { inner }
        }
    }

    impl<H> Calculator for CalculatorProxy<H>
    where
        H: ProxyHandler<Counter>,
    {
        fn add(&self, left: i32, right: i32) -> i32 {
            self.inner
                .call("Calculator::add", |target| target.add(left, right))
        }

        fn divide(&self, left: i32, right: i32) -> Result<i32, &'static str> {
            self.inner
                .invoke("Calculator::divide", |target| target.divide(left, right))
        }
    }

    #[derive(Clone, Default)]
    struct RecordingHandler {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingHandler {
        fn snapshot(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    impl ProxyHandler<Counter> for RecordingHandler {
        fn before(&self, _target: &Counter, method: &str, _call_id: u64) {
            self.events.lock().unwrap().push(format!("before:{method}"));
        }

        fn after<R>(&self, _target: &Counter, method: &str, _call_id: u64, _result: &R) {
            self.events.lock().unwrap().push(format!("after:{method}"));
        }

        fn on_error<E>(&self, _target: &Counter, method: &str, _call_id: u64, _error: &E) {
            self.events.lock().unwrap().push(format!("error:{method}"));
        }
    }

    #[test]
    fn proxy_runs_sync_hooks() {
        let handler = RecordingHandler::default();
        let proxy = DefaultProxy::with_handler(Counter { value: 2 }, handler.clone());

        let result = proxy.call("double", |target| target.value * 2);

        assert_eq!(result, 4);
        assert_eq!(
            handler.snapshot(),
            vec!["before:double".to_string(), "after:double".to_string()]
        );
    }

    #[test]
    fn proxy_runs_error_hook() {
        let handler = RecordingHandler::default();
        let proxy = DefaultProxy::with_handler(Counter { value: 1 }, handler.clone());

        let result = proxy.invoke::<(), _, _>("fail", |_target| Err("boom"));

        assert_eq!(result, Err("boom"));
        assert_eq!(
            handler.snapshot(),
            vec!["before:fail".to_string(), "error:fail".to_string()]
        );
    }

    #[test]
    fn proxy_supports_mutable_calls() {
        let handler = RecordingHandler::default();
        let mut proxy = DefaultProxy::with_handler(Counter::default(), handler.clone());

        let result = proxy.call_mut("increment", |target| {
            target.value += 1;
            target.value
        });

        assert_eq!(result, 1);
        assert_eq!(proxy.target().value, 1);
        assert_eq!(
            handler.snapshot(),
            vec![
                "before:increment".to_string(),
                "after:increment".to_string()
            ]
        );
    }

    #[test]
    fn composite_handler_preserves_aop_order() {
        let first = RecordingHandler::default();
        let second = RecordingHandler::default();
        let handler = CompositeProxyHandler::new(first.clone(), second.clone());
        let proxy = DefaultProxy::with_handler(Counter { value: 0 }, handler);

        let result = proxy.call("sum", |_target| 42);

        assert_eq!(result, 42);
        assert_eq!(
            first.snapshot(),
            vec!["before:sum".to_string(), "after:sum".to_string()]
        );
        assert_eq!(
            second.snapshot(),
            vec!["before:sum".to_string(), "after:sum".to_string()]
        );
    }

    #[test]
    fn trait_wrapper_can_delegate_through_proxy() {
        let handler = RecordingHandler::default();
        let calculator = CalculatorProxy::new(DefaultProxy::with_handler(
            Counter { value: 1 },
            handler.clone(),
        ));

        assert_eq!(calculator.add(2, 3), 6);
        assert_eq!(calculator.divide(8, 2), Ok(4));
        assert_eq!(calculator.divide(8, 0), Err("division by zero"));
        assert_eq!(
            handler.snapshot(),
            vec![
                "before:Calculator::add".to_string(),
                "after:Calculator::add".to_string(),
                "before:Calculator::divide".to_string(),
                "after:Calculator::divide".to_string(),
                "before:Calculator::divide".to_string(),
                "error:Calculator::divide".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn proxy_supports_async_calls() {
        let handler = RecordingHandler::default();
        let proxy = DefaultProxy::with_handler(Counter { value: 7 }, handler.clone());

        let result = proxy
            .invoke_async("async_double", |target| {
                let value = target.value;
                async move { Ok::<_, &'static str>(value * 2) }
            })
            .await;

        assert_eq!(result, Ok(14));
        assert_eq!(
            handler.snapshot(),
            vec![
                "before:async_double".to_string(),
                "after:async_double".to_string()
            ]
        );
    }
}
