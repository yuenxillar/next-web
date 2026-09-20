//! Bounded, resizable executor for blocking work.

use std::cell::Cell;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::task::{Context, Poll};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

type Job = Box<dyn FnOnce() + Send + 'static>;
static NEXT_POOL_ID: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static CURRENT_POOL_ID: Cell<Option<usize>> = const { Cell::new(None) };
}

struct Task {
    name: Option<String>,
    job: Job,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Lifecycle {
    Running,
    ShuttingDown,
    Stopping,
    Terminated,
}

/// The externally visible lifecycle of an executor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutorState {
    Running,
    ShuttingDown,
    Stopping,
    Terminated,
}

impl From<Lifecycle> for ExecutorState {
    fn from(value: Lifecycle) -> Self {
        match value {
            Lifecycle::Running => Self::Running,
            Lifecycle::ShuttingDown => Self::ShuttingDown,
            Lifecycle::Stopping => Self::Stopping,
            Lifecycle::Terminated => Self::Terminated,
        }
    }
}

struct State {
    lifecycle: Lifecycle,
    queue: VecDeque<Task>,
    active: usize,
    live_workers: usize,
    desired_workers: usize,
    completed: u64,
    panicked: u64,
    rejected: u64,
    cancelled: u64,
}

struct Inner {
    id: usize,
    name: Option<String>,
    name_is_prefix: bool,
    stack_size: Option<usize>,
    queue_capacity: usize,
    state: Mutex<State>,
    work_available: Condvar,
    queue_space: Condvar,
    idle: Condvar,
    terminated: Condvar,
    management: Mutex<()>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    next_worker_id: AtomicUsize,
    external_handles: AtomicUsize,
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn is_current_worker(&self) -> bool {
        CURRENT_POOL_ID.with(|id| id.get() == Some(self.id))
    }

    fn notify_idle(&self, state: &State) {
        if state.queue.is_empty() && state.active == 0 {
            self.idle.notify_all();
        }
    }
}

/// Why a task was not accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecuteError {
    QueueFull,
    TimedOut,
    ShuttingDown,
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::QueueFull => "thread-pool task queue is full",
            Self::TimedOut => "timed out waiting for thread-pool queue capacity",
            Self::ShuttingDown => "thread-pool executor is shutting down",
        })
    }
}
impl Error for ExecuteError {}

/// An operation would deadlock because it was called by this pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaitError {
    CalledFromWorker,
}

impl fmt::Display for WaitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a worker cannot wait for its own thread-pool executor")
    }
}
impl Error for WaitError {}

/// Failure returned while awaiting a submitted task's value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskError {
    /// The task closure panicked. The worker remains healthy.
    Panicked,
    /// The task was discarded by immediate shutdown before it started.
    Cancelled,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Panicked => "thread-pool task panicked",
            Self::Cancelled => "thread-pool task was cancelled",
        })
    }
}
impl Error for TaskError {}

/// An awaitable result from [`ThreadPoolTaskExecutor::submit`].
#[must_use = "dropping the handle detaches the task; its work still runs"]
pub struct TaskHandle<T> {
    receiver: oneshot::Receiver<Result<T, TaskError>>,
}

impl<T> Future for TaskHandle<T> {
    type Output = Result<T, TaskError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.receiver).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(TaskError::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A consistent point-in-time executor snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutorMetrics {
    pub state: ExecutorState,
    pub queued_tasks: usize,
    pub active_tasks: usize,
    pub live_workers: usize,
    pub desired_workers: usize,
    pub completed_tasks: u64,
    pub panicked_tasks: u64,
    pub rejected_tasks: u64,
    pub cancelled_tasks: u64,
}

/// Configures a [`ThreadPoolTaskExecutor`].
#[derive(Clone, Debug)]
pub struct Builder {
    num_threads: Option<usize>,
    thread_name: Option<String>,
    name_is_prefix: bool,
    stack_size: Option<usize>,
    queue_capacity: usize,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            num_threads: None,
            thread_name: None,
            name_is_prefix: false,
            stack_size: None,
            queue_capacity: 1_024,
        }
    }
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn num_threads(mut self, value: usize) -> Self {
        assert!(value > 0, "num_threads must be greater than zero");
        self.num_threads = Some(value);
        self
    }

    /// Gives every worker the same name (legacy behavior).
    pub fn thread_name(mut self, value: String) -> Self {
        self.thread_name = Some(value);
        self.name_is_prefix = false;
        self
    }

    /// Names workers using the prefix followed by a numeric worker ID.
    pub fn thread_name_prefix(mut self, value: impl Into<String>) -> Self {
        self.thread_name = Some(value.into());
        self.name_is_prefix = true;
        self
    }

    pub fn thread_stack_size(mut self, value: usize) -> Self {
        self.stack_size = Some(value);
        self
    }

    /// Sets the bounded waiting queue capacity.
    pub fn queue_capacity(mut self, value: usize) -> Self {
        assert!(value > 0, "queue_capacity must be greater than zero");
        self.queue_capacity = value;
        self
    }

    pub fn build(self) -> ThreadPoolTaskExecutor {
        self.try_build()
            .expect("unable to create thread-pool workers")
    }

    pub fn try_build(self) -> io::Result<ThreadPoolTaskExecutor> {
        let n = self.num_threads.unwrap_or_else(default_parallelism);
        let inner = Arc::new(Inner {
            id: NEXT_POOL_ID.fetch_add(1, Ordering::Relaxed),
            name: self.thread_name,
            name_is_prefix: self.name_is_prefix,
            stack_size: self.stack_size,
            queue_capacity: self.queue_capacity,
            state: Mutex::new(State {
                lifecycle: Lifecycle::Running,
                queue: VecDeque::with_capacity(self.queue_capacity),
                active: 0,
                live_workers: 0,
                desired_workers: n,
                completed: 0,
                panicked: 0,
                rejected: 0,
                cancelled: 0,
            }),
            work_available: Condvar::new(),
            queue_space: Condvar::new(),
            idle: Condvar::new(),
            terminated: Condvar::new(),
            management: Mutex::new(()),
            workers: Mutex::new(Vec::with_capacity(n)),
            next_worker_id: AtomicUsize::new(1),
            external_handles: AtomicUsize::new(1),
        });
        let executor = ThreadPoolTaskExecutor { inner };
        if let Err(error) = executor.spawn_to_target() {
            executor.shutdown_now();
            let _ = executor.await_termination(Duration::from_secs(5));
            return Err(error);
        }
        Ok(executor)
    }
}

fn default_parallelism() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

/// A bounded, dynamically resizable executor intended for blocking web work.
///
/// Clones share one executor. Dropping the final handle starts graceful
/// shutdown without blocking. For deterministic shutdown call [`Self::shutdown`]
/// and then [`Self::await_termination`].
pub struct ThreadPoolTaskExecutor {
    inner: Arc<Inner>,
}

/// Backward-compatible name.
pub type ThreadPool = ThreadPoolTaskExecutor;

impl ThreadPoolTaskExecutor {
    pub fn new(num_threads: usize) -> Self {
        Builder::new().num_threads(num_threads).build()
    }

    pub fn with_name(name: String, num_threads: usize) -> Self {
        Builder::new()
            .num_threads(num_threads)
            .thread_name(name)
            .build()
    }

    /// Non-blocking submission. A full queue is reported as backpressure.
    pub fn execute<F>(&self, job: F) -> Result<(), ExecuteError>
    where
        F: FnOnce() + Send + 'static,
    {
        self.try_execute(job)
    }

    pub fn try_execute<F>(&self, job: F) -> Result<(), ExecuteError>
    where
        F: FnOnce() + Send + 'static,
    {
        self.enqueue(Task {
            name: None,
            job: Box::new(job),
        })
    }

    pub fn try_execute_named<F>(&self, name: impl Into<String>, job: F) -> Result<(), ExecuteError>
    where
        F: FnOnce() + Send + 'static,
    {
        self.enqueue(Task {
            name: Some(name.into()),
            job: Box::new(job),
        })
    }

    /// Submits blocking work and returns a handle that can be awaited without
    /// blocking an async runtime worker.
    pub fn submit<F, T>(&self, job: F) -> Result<TaskHandle<T>, ExecuteError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.submit_task(None, job)
    }

    /// Like [`Self::submit`], but includes a name in panic diagnostics.
    pub fn submit_named<F, T>(
        &self,
        name: impl Into<String>,
        job: F,
    ) -> Result<TaskHandle<T>, ExecuteError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.submit_task(Some(name.into()), job)
    }

    fn submit_task<F, T>(&self, name: Option<String>, job: F) -> Result<TaskHandle<T>, ExecuteError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let task = move || match panic::catch_unwind(AssertUnwindSafe(job)) {
            Ok(value) => {
                let _ = sender.send(Ok(value));
            }
            Err(payload) => {
                let _ = sender.send(Err(TaskError::Panicked));
                panic::resume_unwind(payload);
            }
        };
        self.enqueue(Task {
            name,
            job: Box::new(task),
        })?;
        Ok(TaskHandle { receiver })
    }

    fn enqueue(&self, task: Task) -> Result<(), ExecuteError> {
        let mut state = self.inner.state();
        if state.lifecycle != Lifecycle::Running {
            state.rejected = state.rejected.saturating_add(1);
            return Err(ExecuteError::ShuttingDown);
        }
        if state.queue.len() >= self.inner.queue_capacity {
            state.rejected = state.rejected.saturating_add(1);
            return Err(ExecuteError::QueueFull);
        }
        state.queue.push_back(task);
        self.inner.work_available.notify_one();
        Ok(())
    }

    /// Waits for queue capacity. Avoid this on async runtime workers.
    pub fn execute_timeout<F>(&self, timeout: Duration, job: F) -> Result<(), ExecuteError>
    where
        F: FnOnce() + Send + 'static,
    {
        let deadline = Instant::now() + timeout;
        let mut task = Some(Task {
            name: None,
            job: Box::new(job),
        });
        let mut state = self.inner.state();
        loop {
            if state.lifecycle != Lifecycle::Running {
                state.rejected = state.rejected.saturating_add(1);
                return Err(ExecuteError::ShuttingDown);
            }
            if state.queue.len() < self.inner.queue_capacity {
                state
                    .queue
                    .push_back(task.take().expect("task enqueued once"));
                self.inner.work_available.notify_one();
                return Ok(());
            }
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                state.rejected = state.rejected.saturating_add(1);
                return Err(ExecuteError::TimedOut);
            };
            let (next, result) = self
                .inner
                .queue_space
                .wait_timeout(state, left)
                .unwrap_or_else(|e| e.into_inner());
            state = next;
            if result.timed_out() && state.queue.len() >= self.inner.queue_capacity {
                state.rejected = state.rejected.saturating_add(1);
                return Err(ExecuteError::TimedOut);
            }
        }
    }

    /// Changes worker count; shrinking never interrupts active tasks.
    pub fn set_num_threads(&self, value: usize) -> io::Result<()> {
        if value == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "num_threads must be greater than zero",
            ));
        }
        let _manager = self
            .inner
            .management
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        {
            let mut state = self.inner.state();
            if state.lifecycle != Lifecycle::Running {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "cannot resize a shutting-down executor",
                ));
            }
            state.desired_workers = value;
            self.inner.work_available.notify_all();
        }
        if let Err(error) = self.spawn_to_target_locked() {
            let mut state = self.inner.state();
            state.desired_workers = state.live_workers.max(1);
            return Err(error);
        }
        self.reap_finished();
        Ok(())
    }

    #[deprecated(since = "0.2.1", note = "use set_num_threads")]
    pub fn set_threads(&self, value: usize) -> io::Result<()> {
        self.set_num_threads(value)
    }

    fn spawn_to_target(&self) -> io::Result<()> {
        let _manager = self
            .inner
            .management
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.spawn_to_target_locked()
    }

    fn spawn_to_target_locked(&self) -> io::Result<()> {
        loop {
            let worker_id = {
                let mut state = self.inner.state();
                if state.lifecycle != Lifecycle::Running
                    || state.live_workers >= state.desired_workers
                {
                    return Ok(());
                }
                state.live_workers += 1;
                self.inner.next_worker_id.fetch_add(1, Ordering::Relaxed)
            };
            let mut builder = thread::Builder::new();
            if let Some(name) = &self.inner.name {
                builder = builder.name(if self.inner.name_is_prefix {
                    format!("{name}{worker_id}")
                } else {
                    name.clone()
                });
            }
            if let Some(size) = self.inner.stack_size {
                builder = builder.stack_size(size);
            }
            let inner = Arc::clone(&self.inner);
            match builder.spawn(move || worker_loop(inner)) {
                Ok(handle) => self
                    .inner
                    .workers
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(handle),
                Err(error) => {
                    let mut state = self.inner.state();
                    state.live_workers = state.live_workers.saturating_sub(1);
                    return Err(error);
                }
            }
        }
    }

    /// Waits for a moment at which the queue and active set are empty.
    pub fn join(&self) -> Result<(), WaitError> {
        if self.inner.is_current_worker() {
            return Err(WaitError::CalledFromWorker);
        }
        let mut state = self.inner.state();
        while !state.queue.is_empty() || state.active != 0 {
            state = self
                .inner
                .idle
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }
        Ok(())
    }

    pub fn wait_for_idle(&self, timeout: Duration) -> Result<bool, WaitError> {
        if self.inner.is_current_worker() {
            return Err(WaitError::CalledFromWorker);
        }
        let deadline = Instant::now() + timeout;
        let mut state = self.inner.state();
        while !state.queue.is_empty() || state.active != 0 {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return Ok(false);
            };
            let (next, result) = self
                .inner
                .idle
                .wait_timeout(state, left)
                .unwrap_or_else(|e| e.into_inner());
            state = next;
            if result.timed_out() && (!state.queue.is_empty() || state.active != 0) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Stops accepting work and completes queued tasks.
    pub fn shutdown(&self) {
        let _manager = self
            .inner
            .management
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut state = self.inner.state();
        if state.lifecycle == Lifecycle::Running {
            state.lifecycle = Lifecycle::ShuttingDown;
            state.desired_workers = 0;
            if state.live_workers == 0 {
                state.lifecycle = Lifecycle::Terminated;
                self.inner.terminated.notify_all();
            }
            self.inner.work_available.notify_all();
            self.inner.queue_space.notify_all();
        }
    }

    /// Discards queued work. Active Rust threads are never forcefully killed.
    pub fn shutdown_now(&self) -> usize {
        let cancelled = {
            let _manager = self
                .inner
                .management
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut state = self.inner.state();
            if state.lifecycle == Lifecycle::Terminated {
                return 0;
            }
            state.lifecycle = Lifecycle::Stopping;
            state.desired_workers = 0;
            let tasks: Vec<_> = state.queue.drain(..).collect();
            state.cancelled = state.cancelled.saturating_add(tasks.len() as u64);
            self.inner.notify_idle(&state);
            if state.live_workers == 0 {
                state.lifecycle = Lifecycle::Terminated;
                self.inner.terminated.notify_all();
            }
            self.inner.work_available.notify_all();
            self.inner.queue_space.notify_all();
            tasks
        };
        let count = cancelled.len();
        drop(cancelled);
        count
    }

    /// Waits for shutdown and joins worker OS handles.
    pub fn await_termination(&self, timeout: Duration) -> bool {
        if self.inner.is_current_worker() {
            return false;
        }
        let deadline = Instant::now() + timeout;
        let mut state = self.inner.state();
        while state.lifecycle != Lifecycle::Terminated {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            let (next, result) = self
                .inner
                .terminated
                .wait_timeout(state, left)
                .unwrap_or_else(|e| e.into_inner());
            state = next;
            if result.timed_out() && state.lifecycle != Lifecycle::Terminated {
                return false;
            }
        }
        drop(state);
        self.join_all();
        true
    }

    pub fn metrics(&self) -> ExecutorMetrics {
        let s = self.inner.state();
        ExecutorMetrics {
            state: s.lifecycle.into(),
            queued_tasks: s.queue.len(),
            active_tasks: s.active,
            live_workers: s.live_workers,
            desired_workers: s.desired_workers,
            completed_tasks: s.completed,
            panicked_tasks: s.panicked,
            rejected_tasks: s.rejected,
            cancelled_tasks: s.cancelled,
        }
    }

    pub fn queued_count(&self) -> usize {
        self.metrics().queued_tasks
    }
    pub fn active_count(&self) -> usize {
        self.metrics().active_tasks
    }
    pub fn max_count(&self) -> usize {
        self.metrics().desired_workers
    }
    pub fn worker_count(&self) -> usize {
        self.metrics().live_workers
    }
    pub fn panic_count(&self) -> usize {
        self.metrics()
            .panicked_tasks
            .try_into()
            .unwrap_or(usize::MAX)
    }
    pub fn queue_capacity(&self) -> usize {
        self.inner.queue_capacity
    }
    pub fn state(&self) -> ExecutorState {
        self.metrics().state
    }

    fn reap_finished(&self) {
        let done = {
            let mut handles = self.inner.workers.lock().unwrap_or_else(|e| e.into_inner());
            let mut done = Vec::new();
            let mut alive = Vec::new();
            for h in handles.drain(..) {
                if h.is_finished() {
                    done.push(h)
                } else {
                    alive.push(h)
                }
            }
            *handles = alive;
            done
        };
        for h in done {
            let _ = h.join();
        }
    }

    fn join_all(&self) {
        let handles =
            std::mem::take(&mut *self.inner.workers.lock().unwrap_or_else(|e| e.into_inner()));
        for h in handles {
            let _ = h.join();
        }
    }
}

impl Clone for ThreadPoolTaskExecutor {
    fn clone(&self) -> Self {
        self.inner.external_handles.fetch_add(1, Ordering::Relaxed);
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Drop for ThreadPoolTaskExecutor {
    fn drop(&mut self) {
        if self.inner.external_handles.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.shutdown();
        }
    }
}

impl Default for ThreadPoolTaskExecutor {
    fn default() -> Self {
        Builder::new().build()
    }
}
impl PartialEq for ThreadPoolTaskExecutor {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}
impl Eq for ThreadPoolTaskExecutor {}
impl fmt::Debug for ThreadPoolTaskExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadPoolTaskExecutor")
            .field("name", &self.inner.name)
            .field("queue_capacity", &self.inner.queue_capacity)
            .field("metrics", &self.metrics())
            .finish()
    }
}

fn worker_loop(inner: Arc<Inner>) {
    CURRENT_POOL_ID.with(|id| id.set(Some(inner.id)));
    loop {
        let task = {
            let mut state = inner.state();
            loop {
                let retire = match state.lifecycle {
                    Lifecycle::Running => state.live_workers > state.desired_workers,
                    Lifecycle::ShuttingDown => state.queue.is_empty(),
                    Lifecycle::Stopping | Lifecycle::Terminated => true,
                };
                if retire {
                    retire_worker(&inner, &mut state);
                    CURRENT_POOL_ID.with(|id| id.set(None));
                    return;
                }
                if let Some(task) = state.queue.pop_front() {
                    state.active += 1;
                    inner.queue_space.notify_one();
                    break task;
                }
                state = inner
                    .work_available
                    .wait(state)
                    .unwrap_or_else(|e| e.into_inner());
            }
        };
        let name = task.name;
        let result = panic::catch_unwind(AssertUnwindSafe(task.job));
        let panic_message = result
            .as_ref()
            .err()
            .map(|payload| panic_message(payload.as_ref()).to_owned());
        let mut state = inner.state();
        state.active = state.active.saturating_sub(1);
        state.completed = state.completed.saturating_add(1);
        if panic_message.is_some() {
            state.panicked = state.panicked.saturating_add(1);
        }
        inner.notify_idle(&state);
        drop(state);
        if let Some(message) = panic_message {
            tracing::error!(
                task.name = name.as_deref().unwrap_or("<unnamed>"),
                panic.message = %message,
                "thread-pool task panicked"
            );
        }
    }
}

fn retire_worker(inner: &Inner, state: &mut State) {
    state.live_workers = state.live_workers.saturating_sub(1);
    if state.live_workers == 0 && state.lifecycle != Lifecycle::Running {
        state.lifecycle = Lifecycle::Terminated;
        inner.terminated.notify_all();
    }
    inner.notify_idle(state);
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Barrier};

    const TIMEOUT: Duration = Duration::from_secs(2);

    #[test]
    fn executes_and_isolates_panics() {
        let pool = ThreadPoolTaskExecutor::new(2);
        pool.try_execute_named("expected", || panic!("boom"))
            .unwrap();
        let (tx, rx) = mpsc::channel();
        pool.execute(move || tx.send(42).unwrap()).unwrap();
        assert_eq!(rx.recv_timeout(TIMEOUT).unwrap(), 42);
        pool.join().unwrap();
        assert_eq!(pool.panic_count(), 1);
        assert_eq!(pool.worker_count(), 2);
    }

    #[test]
    fn submitted_values_are_awaitable() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let pool = ThreadPoolTaskExecutor::new(1);
        let value = runtime.block_on(pool.submit(|| 6 * 7).unwrap()).unwrap();
        assert_eq!(value, 42);
        let error = runtime
            .block_on(
                pool.submit_named("expected", || -> usize { panic!("boom") })
                    .unwrap(),
            )
            .unwrap_err();
        assert_eq!(error, TaskError::Panicked);
        pool.join().unwrap();
        assert_eq!(pool.panic_count(), 1);
    }

    #[test]
    fn bounded_queue_applies_backpressure() {
        let pool = Builder::new().num_threads(1).queue_capacity(1).build();
        let started = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let (s, r) = (started.clone(), release.clone());
        pool.execute(move || {
            s.wait();
            r.wait();
        })
        .unwrap();
        started.wait();
        pool.execute(|| {}).unwrap();
        assert_eq!(pool.execute(|| {}), Err(ExecuteError::QueueFull));
        release.wait();
        pool.join().unwrap();
    }

    #[test]
    fn timed_submission_and_queued_handle_cancellation_are_reported() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let pool = Builder::new().num_threads(1).queue_capacity(1).build();
        let started = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let (s, r) = (started.clone(), release.clone());
        pool.execute(move || {
            s.wait();
            r.wait();
        })
        .unwrap();
        started.wait();
        let handle = pool.submit(|| 42).unwrap();
        assert_eq!(
            pool.execute_timeout(Duration::from_millis(10), || {}),
            Err(ExecuteError::TimedOut)
        );
        assert_eq!(pool.shutdown_now(), 1);
        assert_eq!(runtime.block_on(handle), Err(TaskError::Cancelled));
        release.wait();
        assert!(pool.await_termination(TIMEOUT));
    }

    #[test]
    fn shrink_and_expand_tracks_real_workers() {
        let pool = ThreadPoolTaskExecutor::new(4);
        pool.set_num_threads(1).unwrap();
        assert!(wait_until(|| pool.worker_count() == 1));
        pool.set_num_threads(3).unwrap();
        assert!(wait_until(|| pool.worker_count() == 3));
    }

    #[test]
    fn graceful_and_immediate_shutdown_are_distinct() {
        let pool = Builder::new().num_threads(1).queue_capacity(8).build();
        let started = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let (s, r) = (started.clone(), release.clone());
        pool.execute(move || {
            s.wait();
            r.wait();
        })
        .unwrap();
        started.wait();
        for _ in 0..5 {
            pool.execute(|| {}).unwrap();
        }
        assert_eq!(pool.shutdown_now(), 5);
        release.wait();
        assert!(pool.await_termination(TIMEOUT));
        assert_eq!(pool.state(), ExecutorState::Terminated);

        let pool = ThreadPoolTaskExecutor::new(1);
        let (tx, rx) = mpsc::channel();
        for _ in 0..4 {
            let tx = tx.clone();
            pool.execute(move || tx.send(()).unwrap()).unwrap();
        }
        drop(tx);
        pool.shutdown();
        assert!(pool.await_termination(TIMEOUT));
        assert_eq!(rx.iter().count(), 4);
    }

    #[test]
    fn self_join_is_rejected() {
        let pool = ThreadPoolTaskExecutor::new(1);
        let worker_pool = pool.clone();
        let (tx, rx) = mpsc::channel();
        pool.execute(move || tx.send(worker_pool.join()).unwrap())
            .unwrap();
        assert_eq!(
            rx.recv_timeout(TIMEOUT).unwrap(),
            Err(WaitError::CalledFromWorker)
        );
    }

    fn wait_until(test: impl Fn() -> bool) -> bool {
        let end = Instant::now() + TIMEOUT;
        while Instant::now() < end {
            if test() {
                return true;
            }
            thread::sleep(Duration::from_millis(2));
        }
        test()
    }
}
