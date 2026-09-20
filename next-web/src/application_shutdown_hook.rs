//! Application shutdown hook.
//!
//! [`ApplicationShutdownHook`] is a small coordination primitive that lets any
//! number of callbacks be registered and invoked when the application shuts
//! down. Every registered callback is notified through a shared
//! [`CancellationToken`], so long running tasks can observe the shutdown
//! request and stop gracefully while the callbacks release their resources.
//!
//! The hook is idempotent: no matter how many times [`ApplicationShutdownHook::shutdown`]
//! is called, the registered callbacks run exactly once and in registration
//! order.
//!
//! # Example
//!
//! ```
//! use next_web::ApplicationShutdownHook;
//!
//! let hook = ApplicationShutdownHook::new();
//! let token = hook.cancellation_token();
//!
//! hook.add_hook(move || async move {
//!     // Release resources, flush buffers, ...
//! });
//!
//! assert!(!token.is_cancelled());
//! assert_eq!(hook.hook_count(), 1);
//! ```

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use tokio_util::sync::CancellationToken;

/// A boxed, owned future produced by a shutdown hook.
pub type ShutdownFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// A shutdown callback that is invoked at most once.
type ShutdownCallback = Box<dyn FnOnce() -> ShutdownFuture + Send + 'static>;

/// Shared state backing every [`ApplicationShutdownHook`] clone.
#[derive(Default)]
struct ApplicationShutdownHookInner {
    /// Registered callbacks paired with their registration id, in order.
    callbacks: Mutex<Vec<(usize, ShutdownCallback)>>,
    /// Monotonic id source used to hand out stable callback identifiers.
    next_id: AtomicUsize,
    /// Guards against running the callbacks more than once.
    triggered: AtomicBool,
}

/// Coordinates application shutdown across multiple registered hooks.
///
/// A hook is any callback that returns a [`Future`]. Register as many as
/// needed with [`Self::add_hook`] (async) or [`Self::add_hook_sync`]
/// (blocking), then trigger them all with [`Self::shutdown`].
///
/// The same [`CancellationToken`] is shared by every clone and can be handed
/// to background tasks so they can react to the shutdown request, for example
/// by selecting on [`Self::cancelled`].
///
/// Cloning an [`ApplicationShutdownHook`] produces a handle that shares the
/// exact same token and callback registry.
#[derive(Clone, Default)]
pub struct ApplicationShutdownHook {
    token: CancellationToken,
    inner: Arc<ApplicationShutdownHookInner>,
}

impl ApplicationShutdownHook {
    /// Creates a new, empty shutdown hook.
    ///
    /// # Returns
    ///
    /// A shutdown hook with no registered callbacks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a clone of the [`CancellationToken`] used to coordinate
    /// shutdown.
    ///
    /// The token is cancelled as soon as [`Self::shutdown`] is invoked, before
    /// any callback runs. Tasks can await [`Self::cancelled`] or call
    /// `token.cancelled().await` to observe the request.
    ///
    /// # Returns
    ///
    /// A shared cancellation token.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Returns whether shutdown has already been requested.
    ///
    /// # Returns
    ///
    /// `true` once the shared cancellation token has been cancelled.
    pub fn is_shutdown(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Resolves once shutdown has been requested.
    ///
    /// This is a convenience wrapper around the shared token's
    /// `cancelled()` future.
    pub async fn cancelled(&self) {
        self.token.cancelled().await;
    }

    /// Registers an asynchronous shutdown hook.
    ///
    /// The provided closure is invoked exactly once when [`Self::shutdown`] is
    /// called. Hooks run in the order they were registered, and a panic inside
    /// one hook does not prevent the remaining hooks from running.
    ///
    /// # Arguments
    ///
    /// * `hook` - A closure returning the future to run on shutdown.
    ///
    /// # Returns
    ///
    /// A stable identifier that can be passed to [`Self::remove_hook`].
    ///
    /// # Example
    ///
    /// ```
    /// # use next_web::ApplicationShutdownHook;
    /// let hook = ApplicationShutdownHook::new();
    /// let id = hook.add_hook(|| async {
    ///     // ...
    /// });
    /// assert!(hook.remove_hook(id));
    /// ```
    pub fn add_hook<F, Fut>(&self, hook: F) -> usize
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.register(Box::new(move || Box::pin(hook())))
    }

    /// Registers a synchronous shutdown hook.
    ///
    /// This is a convenience wrapper for blocking callbacks that do not need to
    /// await anything.
    ///
    /// # Arguments
    ///
    /// * `hook` - A blocking closure to run on shutdown.
    ///
    /// # Returns
    ///
    /// A stable identifier that can be passed to [`Self::remove_hook`].
    pub fn add_hook_sync<F>(&self, hook: F) -> usize
    where
        F: FnOnce() + Send + 'static,
    {
        self.add_hook(move || {
            hook();
            std::future::ready(())
        })
    }

    /// Registers a pre-built future to be awaited on shutdown.
    ///
    /// The future is created eagerly but only polled when [`Self::shutdown`]
    /// runs.
    ///
    /// # Arguments
    ///
    /// * `future` - The future to await on shutdown.
    ///
    /// # Returns
    ///
    /// A stable identifier that can be passed to [`Self::remove_hook`].
    pub fn add_future<Fut>(&self, future: Fut) -> usize
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.add_hook(move || future)
    }

    /// Removes a previously registered hook.
    ///
    /// Removing a hook that has already run or was never registered is a no-op.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier returned when the hook was registered.
    ///
    /// # Returns
    ///
    /// `true` if a hook with the given identifier was removed.
    pub fn remove_hook(&self, id: usize) -> bool {
        let mut callbacks = self.lock_callbacks();
        let before = callbacks.len();
        callbacks.retain(|(callback_id, _)| *callback_id != id);
        callbacks.len() != before
    }

    /// Returns the number of currently registered hooks.
    ///
    /// # Returns
    ///
    /// The number of hooks that have not been removed or run yet.
    pub fn hook_count(&self) -> usize {
        self.lock_callbacks().len()
    }

    /// Requests shutdown, cancelling the shared token and running every
    /// registered hook.
    ///
    /// This method is idempotent: only the first invocation runs the hooks.
    /// Subsequent calls return immediately. Hooks are removed before they run,
    /// so they can safely register new hooks from within a callback without
    /// those new hooks being executed by the current shutdown pass.
    pub async fn shutdown(&self) {
        if self.inner.triggered.swap(true, Ordering::SeqCst) {
            return;
        }

        // Notify every observer before running the cleanup callbacks.
        self.token.cancel();

        let callbacks = {
            let mut guard = self.lock_callbacks();
            std::mem::take(&mut *guard)
        };

        for (id, callback) in callbacks {
            if let Err(payload) = AssertUnwindSafe(callback()).catch_unwind().await {
                let message = payload
                    .downcast_ref::<&str>()
                    .map(|value| (*value).to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_string());

                tracing::error!(
                    hook_id = id,
                    "application shutdown hook panicked: {message}"
                );
            }
        }
    }

    /// Registers a callback and returns its identifier.
    fn register(&self, callback: ShutdownCallback) -> usize {
        let id = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        self.lock_callbacks().push((id, callback));
        id
    }

    /// Locks the callback registry, recovering from a poisoned mutex.
    fn lock_callbacks(&self) -> std::sync::MutexGuard<'_, Vec<(usize, ShutdownCallback)>> {
        self.inner
            .callbacks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl std::fmt::Debug for ApplicationShutdownHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplicationShutdownHook")
            .field("hooks", &self.hook_count())
            .field("is_shutdown", &self.is_shutdown())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn runs_all_registered_hooks_in_order() {
        let hook = ApplicationShutdownHook::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        for value in 0..5 {
            let order = Arc::clone(&order);
            hook.add_hook(move || async move {
                order.lock().unwrap().push(value);
            });
        }

        assert_eq!(hook.hook_count(), 5);
        hook.shutdown().await;

        assert_eq!(*order.lock().unwrap(), vec![0, 1, 2, 3, 4]);
        assert!(hook.is_shutdown());
    }

    #[tokio::test]
    async fn shutdown_is_idempotent() {
        let hook = ApplicationShutdownHook::new();
        let runs = Arc::new(AtomicUsize::new(0));

        let counter = Arc::clone(&runs);
        hook.add_hook(move || async move {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        hook.shutdown().await;
        hook.shutdown().await;
        hook.shutdown().await;

        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert_eq!(hook.hook_count(), 0);
    }

    #[tokio::test]
    async fn cancels_token_before_running_hooks() {
        let hook = ApplicationShutdownHook::new();
        let token = hook.cancellation_token();
        let observed = Arc::new(AtomicBool::new(false));

        let observed_clone = Arc::clone(&observed);
        let hook_token = hook.cancellation_token();
        hook.add_hook(move || async move {
            observed_clone.store(hook_token.is_cancelled(), Ordering::SeqCst);
        });

        assert!(!token.is_cancelled());
        hook.shutdown().await;

        assert!(token.is_cancelled());
        assert!(observed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cancelled_resolves_after_shutdown() {
        let hook = ApplicationShutdownHook::new();
        let waiter = hook.clone();

        let handle = tokio::spawn(async move {
            waiter.cancelled().await;
        });

        hook.shutdown().await;
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn supports_sync_and_future_hooks() {
        let hook = ApplicationShutdownHook::new();
        let flag = Arc::new(AtomicBool::new(false));

        let sync_flag = Arc::clone(&flag);
        hook.add_hook_sync(move || {
            sync_flag.store(true, Ordering::SeqCst);
        });

        hook.add_future(async {});

        hook.shutdown().await;
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn remove_hook_prevents_execution() {
        let hook = ApplicationShutdownHook::new();
        let runs = Arc::new(AtomicUsize::new(0));

        let counter = Arc::clone(&runs);
        let id = hook.add_hook(move || async move {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        assert!(hook.remove_hook(id));
        assert!(!hook.remove_hook(id));
        assert_eq!(hook.hook_count(), 0);

        hook.shutdown().await;
        assert_eq!(runs.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn panicking_hook_does_not_stop_others() {
        let hook = ApplicationShutdownHook::new();
        let reached = Arc::new(AtomicBool::new(false));

        hook.add_hook(|| async {
            panic!("boom");
        });

        let reached_clone = Arc::clone(&reached);
        hook.add_hook(move || async move {
            reached_clone.store(true, Ordering::SeqCst);
        });

        hook.shutdown().await;
        assert!(reached.load(Ordering::SeqCst));
    }

    #[test]
    fn clone_shares_token_and_registry() {
        let hook = ApplicationShutdownHook::new();
        let clone = hook.clone();

        clone.add_hook_sync(|| {});

        assert_eq!(hook.hook_count(), 1);
        assert!(!hook.is_shutdown());
        assert!(!clone.is_shutdown());
    }
}
