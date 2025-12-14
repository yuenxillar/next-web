//! # ThreadUtil
//!
//! 一个用于简化线程与异步任务操作的工具结构体。
//! A utility struct for simplifying thread and async task operations.

use std::future::Future;
use std::time::Duration;

use tokio::task::{JoinError, JoinHandle};

/// 线程与异步任务工具类
///
/// 提供对当前线程信息、系统资源、Tokio 运行时任务的便捷访问。
/// Provides convenient access to thread info, system resources, and Tokio runtime tasks.
pub struct ThreadUtil;

impl ThreadUtil {
    /// 获取当前线程的唯一标识 ID。
    ///
    /// # Returns
    ///
    /// 返回一个 `std::thread::ThreadId`，可用于线程间比较或日志追踪。
    /// Returns a `std::thread::ThreadId` for comparison or logging.
    ///
    /// # Example
    ///
    /// ```
    /// use std::thread;
    /// let id = ThreadUtil::id();
    /// println!("Current thread ID: {:?}", id);
    /// ```
    pub fn id() -> std::thread::ThreadId {
        std::thread::current().id()
    }

    /// 获取当前线程的名称（如果已设置）。
    ///
    /// # Returns
    ///
    /// - `Some(name)`：线程有名称，返回 boxed str
    /// - `None`：线程未命名
    ///
    /// Returns `Some(name)` if the thread has a name, otherwise `None`.
    ///
    /// # Note
    ///
    /// 线程名称通常通过 `std::thread::Builder::name()` 设置。
    /// Thread names are usually set via `std::thread::Builder::name()`.
    pub fn name() -> Option<Box<str>> {
        std::thread::current().name().map(|s| s.into())
    }

    /// 判断当前是否运行在 Tokio 异步运行时环境中。
    ///
    /// # Returns
    ///
    /// - `true`：当前在 Tokio 运行时内（可安全使用 `tokio::spawn` 等）
    /// - `false`：不在 Tokio 环境中
    ///
    /// Checks whether the current context is inside a Tokio runtime.
    /// Useful for conditional logic based on execution environment.
    pub fn in_tokio_runtime() -> bool {
        tokio::runtime::Handle::try_current().is_ok()
    }

    /// 捕获当前线程的堆栈回溯（backtrace）。
    ///
    /// # Returns
    ///
    /// 一个 `Backtrace` 对象，可用于调试、错误追踪。
    /// Captures a backtrace of the current call stack.
    ///
    /// # Note
    ///
    /// 需要启用 `backtrace` 功能（默认通常开启）。
    /// Requires `backtrace` feature (enabled by default in most profiles).
    pub fn backtrace() -> std::backtrace::Backtrace {
        std::backtrace::Backtrace::capture()
    }

    /// 获取系统可用的 CPU 并行度（逻辑核心数）。
    ///
    /// # Returns
    ///
    /// 返回操作系统建议的并行任务数，考虑了：
    /// - 实际 CPU 核心数
    /// - 容器资源限制（如 Docker）
    /// - NUMA 架构
    ///
    /// Returns the number of threads that can run concurrently,
    /// taking into account CPU affinity, container limits, etc.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let cores = ThreadUtil::num_cores();
    /// println!("Available parallelism: {}", cores);
    /// ```
    pub fn num_cores() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }

    /// 获取当前 Tokio 运行时的工作线程数量。
    ///
    /// # Returns
    ///
    /// - `Some(n)`：当前在 Tokio 多线程运行时中，有 `n` 个工作线程
    /// - `None`：不在 Tokio 运行时中
    ///
    /// Returns the number of worker threads in the current Tokio runtime.
    /// Returns `None` if not inside a Tokio context.
    pub fn worker_threads() -> Option<usize> {
        tokio::runtime::Handle::try_current()
            .ok()
            .and_then(|handle| Some(handle.metrics().num_workers()))
    }

    /// 获取当前 Tokio 运行时中仍存活的任务总数。
    ///
    /// “存活”指尚未完成或被取消的任务。
    /// Alive tasks include those that are running, pending, or scheduled.
    ///
    /// # Returns
    ///
    /// - `Some(n)`：存活任务数
    /// - `None`：不在 Tokio 运行时中
    ///
    /// Useful for monitoring task pressure or memory usage.
    pub fn alive_tasks() -> Option<usize> {
        tokio::runtime::Handle::try_current()
            .ok()
            .map(|h| h.metrics().num_alive_tasks())
    }
}

impl ThreadUtil {
    /// 在当前 Tokio 运行时中异步地启动一个 `Send` 的 `Future`。
    ///
    /// # Parameters
    ///
    /// - `future`: 满足 `Send + 'static` 的异步任务
    ///
    /// # Returns
    ///
    /// 返回一个 `JoinHandle`，可用于等待结果或取消任务。
    /// Spawns a `Send` future onto the Tokio runtime.
    ///
    /// # Panics
    ///
    /// 如果不在 Tokio 运行时中调用，此函数会 panic。
    /// Will panic if not called within a Tokio runtime.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn example() {
    /// let handle = ThreadUtil::spawn(async { 42 });
    /// let result = handle.await.unwrap();
    /// assert_eq!(result, 42);
    /// # }
    /// ```
    pub async fn spawn<F>(future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        tokio::task::spawn(future)
    }

    /// 在专用的阻塞任务线程池中执行一个同步函数。
    ///
    /// 用于安全运行 CPU 密集型或阻塞 I/O 操作，避免阻塞异步线程。
    /// Runs a blocking function on a dedicated thread pool.
    /// Prevents blocking the async executor.
    ///
    /// # Parameters
    ///
    /// - `f`: 任意 `FnOnce` 同步函数
    ///
    /// # Returns
    ///
    /// `Result<R, JoinError>`：函数返回值或任务被取消的错误。
    ///
    /// # Example
    ///
    /// ```
    /// # async fn example() {
    /// let result = ThreadUtil::spawn_blocking(|| {
    ///     // 模拟耗时计算
    ///     (0..1_000_000).sum::<u64>()
    /// }).await;
    /// println!("Sum: {:?}", result);
    /// # }
    /// ```
    pub async fn spawn_blocking<F, R>(f: F) -> Result<R, JoinError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        tokio::task::spawn_blocking(f).await
    }

    /// 在当前线程上启动一个不可跨越线程的 `Future`（仅限 `!Send` 类型）。
    ///
    /// # Requirements
    ///
    /// - 必须在 `tokio::runtime::Runtime::new().enable_nonblocking().build()?` 创建的运行时中
    /// - 通常用于 `!Send` 类型（如 `Rc`, `RefCell`）
    ///
    /// Spawns a `!Send` future on the current thread.
    /// Useful for UI or single-threaded contexts.
    ///
    /// # Panics
    ///
    /// 如果当前线程不是 Tokio 的本地执行上下文，会 panic。
    pub fn spawn_local<F>(future: F) -> JoinHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        tokio::task::spawn_local(future)
    }

    /// 并发等待一个 `Future` 向量中的所有任务完成。
    ///
    /// 与 `futures::future::join_all` 类似，但自动包装 `tokio::spawn`。
    /// Concurrently waits for all futures in a vector to complete.
    ///
    /// # Parameters
    ///
    /// - `futures`: 一组满足 `Send` 约束的异步任务
    ///
    /// # Returns
    ///
    /// 返回每个任务的 `Result<T, JoinError>` 向量，顺序与输入一致。
    ///
    /// # Note
    ///
    /// 所有任务同时开始，彼此独立。
    pub async fn join_all<F, T>(futures: Vec<F>) -> Vec<Result<T, JoinError>>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let handles: Vec<_> = futures.into_iter().map(tokio::task::spawn).collect();
        futures::future::join_all(handles).await
    }

    /// 异步睡眠指定时长。
    ///
    /// # Parameters
    ///
    /// - `dur`: 睡眠时间（`std::time::Duration`）
    ///
    /// # Example
    ///
    /// ```
    /// # async fn example() {
    ///     ThreadUtil::sleep(std::time::Duration::from_secs(1)).await;
    ///     println!("Slept for 1 second");
    /// # }
    /// ```
    ///
    /// Internally uses `tokio::time::sleep`, so it's efficient and awaitable.
    pub async fn sleep(dur: Duration) {
        tokio::time::sleep(dur).await;
    }

    /// 主动让出当前任务的执行权，允许其他任务运行。
    ///
    /// # Use Case
    ///
    /// 在长时间运行的异步函数中调用，提高任务调度公平性。
    /// Yields the current task's turn to run, allowing other tasks to proceed.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn long_task() {
    /// for i in 0..1000 {
    ///     // 做一些工作
    ///     if i % 100 == 0 {
    ///         ThreadUtil::yield_now().await; // 让其他任务有机会运行
    ///     }
    /// }
    /// # }
    /// ```
    pub async fn yield_now() {
        tokio::task::yield_now().await;
    }
}