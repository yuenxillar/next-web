//! Helpers the multicaster uses to run the listeners of an event.
//!
//! A listener is asynchronous, but the event system publishes events from
//! synchronous code as well: [`ApplicationEventPublisher`](crate::ApplicationEventPublisher)
//! and therefore every `publish_event` call of an application is synchronous.
//! The helpers of this module bridge the two worlds without requiring an
//! executor of the caller.

use std::{
    any::Any,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread::{self, Thread},
};

/// Runs a future to completion on the current thread.
///
/// The future is polled on the calling thread, and the thread is parked while
/// the future is not ready, so a future that is woken from another thread
/// (a timer of the runtime the application runs on, for example) makes
/// progress. No runtime is started, which is what allows the synchronous API of
/// the event system to run asynchronous listeners.
///
/// A future that can only make progress on the thread it is polled on, such as
/// one of the tasks of a single threaded runtime, cannot complete here. The
/// multicaster therefore only drives listeners inline when the runtime the
/// caller runs on can make progress meanwhile, see
/// [`DefaultApplicationEventMulticaster::multicast_event`](crate::event::DefaultApplicationEventMulticaster::multicast_event).
pub(crate) fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    /// Wakes the thread that waits for the future.
    struct ThreadWaker {
        thread: Thread,
    }

    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.thread.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.thread.unpark();
        }
    }

    let thread = thread::current();
    let waker = Waker::from(Arc::new(ThreadWaker {
        thread: thread.clone(),
    }));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            // `park` returns when the future wakes this thread as well as
            // spuriously, in which case the future is polled again.
            Poll::Pending => thread::park(),
        }
    }
}

/// The result of a future that panicked while it was polled.
pub(crate) type PanicPayload = Box<dyn Any + Send + 'static>;

/// Runs a future and reports a panic of the future instead of unwinding.
///
/// A listener is called by the multicaster, and a listener that panics must not
/// keep the remaining listeners of the same event from running. The wrapper
/// turns the panic into a value, which is what `catch_unwind` does for a
/// function; the future counterpart is implemented here because the panic of a
/// future can only be caught around a call to `poll`.
pub(crate) struct CatchUnwind<F> {
    future: F,
}

impl<F> CatchUnwind<F> {
    /// Wraps `future` so that its panics are reported as errors.
    ///
    /// # Arguments
    ///
    /// * `future` - The future to run.
    pub(crate) fn new(future: F) -> Self {
        Self { future }
    }
}

impl<F> Future for CatchUnwind<F>
where
    F: Future + Unpin,
{
    type Output = Result<F::Output, PanicPayload>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        // Both the wrapper and the future are `Unpin`, so the future can be
        // borrowed mutably without moving it.
        let future = Pin::new(&mut self.future);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| future.poll(context)));

        match result {
            Ok(Poll::Ready(output)) => Poll::Ready(Ok(output)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(payload) => Poll::Ready(Err(payload)),
        }
    }
}

/// Describes a panic payload as a message.
///
/// # Arguments
///
/// * `payload` - The payload of the caught panic.
pub(crate) fn panic_message(payload: &PanicPayload) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }

    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }

    String::from("listener panicked")
}
