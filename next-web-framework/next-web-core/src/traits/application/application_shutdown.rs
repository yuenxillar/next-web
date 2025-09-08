use async_trait::async_trait;
use dyn_clone::DynClone;

use crate::traits::ordered::Ordered;

/// 应用关闭钩子（Application Shutdown）
///
/// 用于在应用程序关闭时执行清理或收尾任务的 trait。
///
/// 当应用开始关闭流程时，该 trait 的实现者将被调用，可用于：
/// - 优雅释放资源（如数据库连接、文件句柄）
/// - 刷新日志或缓存数据
/// - 通知外部系统（如服务注册中心）
/// - 执行异步清理任务（如等待请求完成）
///
/// # 实现要求
///
/// 实现者必须同时实现：
/// - [`DynClone`]：支持动态克隆，便于在运行时复制 trait 对象
/// - [`Ordered`]：定义关闭时的执行优先级，数值越高越早执行
/// - [`Send`] + [`Sync`]：确保可在多线程异步环境中安全使用
///
/// # 示例
///
/// ```rust
/// use next_web_core::traits::application::application_shutdown::ApplicationShutdown;
/// use dyn_clone::DynClone;
/// use next_web_core::traits::ordered::Ordered;
///
/// #[derive(Clone)]
/// struct DatabaseCleaner;
///
/// #[async_trait]
/// impl ApplicationShutdown for DatabaseCleaner {
///     async fn shutdown(&mut self) {
///         log::info!("正在关闭数据库连接...");
///         // 异步关闭连接
///     }
/// }
///
/// ```
///
/// # 执行顺序
///
/// 所有 `ApplicationShutdown` 实例将根据 `Ordered::order()` 的返回值
/// 从高到低排序后依次执行。优先级相同的顺序未定义。
///
/// # 异步支持
///
/// 使用 `#[async_trait]`，因此 `shutdown` 方法支持 `async/await`，
/// 可安全执行异步操作而不会阻塞运行时。
///
/// **注意**：长时间任务应尽量在合理时间内完成，避免无限等待。
///
/// **错误处理**：关闭过程中的错误建议记录日志，但不应中断整体关闭流程。
///
/// Application Shutdown 
///
/// A trait for types that need to perform cleanup or finalization tasks
/// when the application is shutting down.
///
/// This trait is called during the shutdown phase, allowing components
/// to gracefully release resources, flush logs, close connections, or notify
/// external systems before the application terminates.
///
/// # Requirements
///
/// Implementors must also implement:
/// - [`DynClone`] to allow dynamic cloning of trait objects.
/// - [`Ordered`] to define execution order (higher value = earlier).
/// - [`Send`] + [`Sync`] for safe use across threads and async tasks.
///
/// # Example
/// 
/// ```rust
/// use next_web_core::traits::application::application_shutdown::ApplicationShutdown;
/// use dyn_clone::DynClone;
/// use next_web_core::traits::ordered::Ordered;
///
/// #[derive(Clone)]
/// struct DatabaseCleaner;
///
/// #[async_trait]
/// impl ApplicationShutdown for DatabaseCleaner {
///     async fn shutdown(&mut self) {
///         log::info!("Closing database connections...");
///         // Perform async cleanup
///     }
/// }
///
///
/// impl Ordered for DatabaseCleaner {
///     fn order(&self) -> i32 {
///         200
///     }
/// }
/// 
/// ```
///
/// # Execution Order
///
/// All `ApplicationShutdown` instances are sorted by `order()` in descending order
/// and executed sequentially. Order is not guaranteed for equal priorities.
///
/// # Async Support
///
/// Uses `#[async_trait]`, so `shutdown` can safely perform async operations.
///
/// **Note**: Long-running tasks should complete within a reasonable time.
///
/// **Error Handling**: Errors should be logged but not panic, as the application is already terminating.
#[async_trait]
pub trait ApplicationShutdown
where
    Self: DynClone + Ordered,
    Self: Send + Sync,
{
    /// 执行关闭逻辑
    ///
    /// 在应用关闭流程中被调用，用于执行异步清理任务。
    ///
    /// # 注意事项
    ///
    /// - 方法参数为 `&mut self`，允许修改自身状态
    /// - 可安全使用 `async/await`
    /// - 避免无限等待或阻塞操作
    ///
    /// # 错误处理
    ///
    /// 建议记录错误日志，但不应 panic，以免影响其他关闭钩子执行。
    ///
    /// Perform shutdown logic asynchronously.
    ///
    /// Called during application shutdown to perform cleanup tasks.
    ///
    /// # Notes
    ///
    /// - Accepts `&mut self` to allow state mutation
    /// - Safe to use `async/await`
    /// - Avoid infinite waits or blocking calls
    ///
    /// # Error Handling
    ///
    /// Log errors but avoid panicking to ensure other shutdown s run.
    async fn shutdown(&mut self);
}