use async_trait::async_trait;
use dyn_clone::{DynClone, clone_trait_object};

use crate::{ApplicationContext, traits::ordered::Ordered};

/// Trait for monitoring application lifecycle events.
///
/// This trait allows components to hook into key moments of an application's lifecycle:
/// - `on_start`: Called after the application has successfully started
/// - `on_shutdown`: Called when the application is preparing to shut down
///
/// # Examples
///
/// Implementation with error handling and logging:
/// ```
/// use async_trait::async_trait;
/// use next_core::traits::application::application_lifecycle::ApplicationLifecycle;
/// use tracing::{info, error};
///
/// #[derive(Clone)]
/// struct MetricsCollector {
///     endpoint: String,
/// }
///
/// #[async_trait]
/// impl ApplicationLifecycle for MetricsCollector {
///     async fn on_start(&self) -> Result<(), Box<dyn std::error::Error>> {
///         info!("Starting metrics collector to {}", self.endpoint);
///         // Initialize metrics client
///         if self.endpoint.is_empty() {
///             return Err("Metrics endpoint cannot be empty".into());
///         }
///         Ok(())
///     }
///
///     async fn on_shutdown(&self) {
///         info!("Flushing metrics before shutdown...");
///         // Flush any pending metrics
///     }
/// }
/// ```
///
/// # Important Notes
/// - Implementations should be lightweight and non-blocking when possible
/// - `on_shutdown` should not panic or return errors to ensure clean shutdown
/// - The trait is designed to be used with dynamic dispatch (`Box<dyn ApplicationLifecycle>`)
///
///
/// 应用程序生命周期监听 Trait。
///
/// 该 Trait 允许组件在应用程序生命周期的关键时刻执行钩子函数：
/// - `on_start`: 应用程序成功启动后调用
/// - `on_shutdown`: 应用程序准备关闭时调用
///
/// # 示例
///
/// 带有错误处理和日志的实现：
/// ```
/// use async_trait::async_trait;
/// use application_lifecycle::ApplicationLifecycle;
/// use log::{info, error};
///
/// #[derive(Clone)]
/// struct MetricsCollector {
///     endpoint: String,
/// }
///
/// #[async_trait]
/// impl ApplicationLifecycle for MetricsCollector {
///     async fn on_start(&self) -> Result<(), Box<dyn std::error::Error>> {
///         info!("启动指标收集器，上报地址: {}", self.endpoint);
///         // 初始化指标客户端
///         if self.endpoint.is_empty() {
///             return Err("指标上报地址不能为空".into());
///         }
///         Ok(())
///     }
///
///     async fn on_shutdown(&self) {
///         info!("关闭前刷新指标...");
///         // 刷新待上报的指标数据
///     }
/// }
/// ```
///
/// # 重要说明
/// - 实现应尽量轻量且非阻塞
/// - `on_shutdown` 不应 panic 或返回错误，以确保能完成清理
/// - 该 Trait 设计用于动态分发 (`Box<dyn ApplicationLifecycle>`)
///
/// # See also / 参见
/// - [`clone_trait_object`] - Enables cloning of trait objects
/// - [`async_trait`] - Macro for async methods in traits
#[async_trait]
pub trait ApplicationLifecycle
where
    Self: Send + Sync,
    Self: Ordered,
    Self: DynClone,
{
    /// Called after the application has successfully started.
    ///
    /// This method is invoked when all core components are initialized and
    /// the application is ready to begin its main operation. Any initialization
    /// that requires the application to be fully up should be placed here.
    ///
    /// # Returns
    /// - `Ok(())` if initialization was successful
    /// - `Err(Box<dyn std::error::Error>)` if initialization failed
    ///
    /// 应用程序成功启动后调用。
    ///
    /// 当所有核心组件初始化完成，应用程序准备开始主要业务逻辑时，
    /// 会调用此方法。需要应用完全启动后才能执行的初始化操作应放在这里。
    ///
    /// # 返回值
    /// - `Ok(())` 表示初始化成功
    /// - `Err(Box<dyn std::error::Error>)` 表示初始化失败
    async fn on_start(
        &mut self,
        ctx: &mut ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Called when the application is preparing to shut down.
    ///
    /// This method is invoked when a shutdown signal is received (e.g., SIGTERM,
    /// Ctrl+C) or when the application is being gracefully stopped. Implementations
    /// should clean up resources, flush buffers, and perform any necessary
    /// finalization.
    ///
    /// # Arguments
    /// * `ctx` - Context information about the shutdown, including reason and uptime
    ///
    /// # Important
    /// - This method should not panic or return errors to ensure clean shutdown
    /// - Keep operations lightweight to respect shutdown timeouts
    ///
    /// 应用程序准备关闭时调用。
    ///
    /// 当收到关闭信号（如 SIGTERM、Ctrl+C）或应用程序被优雅停止时，
    /// 会调用此方法。实现应清理资源、刷新缓冲区并执行必要的收尾工作。
    ///
    /// # 参数
    /// * `ctx` - 关于关闭的上下文信息，包括原因和运行时长
    ///
    /// # 重要说明
    /// - 此方法不应 panic 或返回错误，以确保能完成清理
    /// - 保持操作轻量，以遵守关闭超时限制
    async fn on_shutdown(&mut self, ctx: &ShutdownContext);
}

#[derive(Debug, Clone)]
pub struct ShutdownContext {
    /// Name of the application being shut down.
    /// Used for logging and identification purposes.
    ///
    /// 正在关闭的应用程序名称。
    /// 用于日志记录和标识目的。
    pub app_name: String,

    /// Total runtime duration from application start to shutdown initiation.
    /// Useful for metrics, monitoring, and debugging.
    ///
    /// 从应用启动到开始关闭的总运行时长。
    /// 用于指标收集、监控和调试。
    pub uptime: std::time::Instant,

    /// Exit code that will be returned to the operating system.
    /// - `Some(0)`: Successful termination
    /// - `Some(n > 0)`: Error termination with specific code
    /// - `None`: Exit code not specified
    ///
    /// 将返回给操作系统的退出码。
    /// - `Some(0)`: 成功终止
    /// - `Some(n > 0)`: 带有特定错误码的错误终止
    /// - `None`: 未指定退出码
    pub exit_code: Option<i32>,

    /// Reason why the application is shutting down.
    /// Provides context about the shutdown trigger for appropriate handling.
    ///
    /// 应用程序关闭的原因。
    /// 提供关闭触发的上下文，以便进行适当的处理。
    pub reason: ShutdownReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShutdownReason {
    /// Normal, expected shutdown (e.g., user requested termination).
    /// No special handling required.
    ///
    /// 正常、预期的关闭（例如，用户请求终止）。
    /// 无需特殊处理。
    Normal,

    /// Shutdown triggered by an operating system signal.
    /// Contains the signal name or number (e.g., "SIGTERM", "SIGINT", "15").
    ///
    /// 由操作系统信号触发的关闭。
    /// 包含信号名称或编号（例如："SIGTERM"、"SIGINT"、"15"）。
    Signal(String),

    /// Shutdown caused by an unrecoverable error.
    /// Contains the error description.
    ///
    /// 由不可恢复的错误导致的关闭。
    /// 包含错误描述。
    Error(String),

    /// Shutdown triggered programmatically through API calls.
    /// Useful for distinguishing between user-initiated and system-initiated
    /// shutdowns in testing or orchestration scenarios.
    ///
    /// 通过 API 调用以编程方式触发的关闭。
    /// 用于在测试或编排场景中区分用户触发和系统触发的关闭。
    Manual,
}

clone_trait_object!(ApplicationLifecycle);
