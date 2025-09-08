use async_trait::async_trait;

use crate::ApplicationContext;

/// **Application Ready Event** (应用就绪事件)
///
/// Notify the relevant implementers when the application completes initialization and is ready
///
/// This event is triggered before bind_tcp_derver
/// Suitable for performing operations such as startup logging, health check notifications, and background task startup.
///
///
/// 当应用程序完成初始化并准备就绪时，通知相关实现者.
///
/// 该事件在 bind_tcp_server 之前触发
/// 适用于执行启动日志记录、健康检查通知、后台任务启动等操作
///
#[async_trait]
pub trait ApplicationReadyEvent: Send + Sync {
    /// Application Ready Event
    async fn ready(&self, ctx: &mut ApplicationContext);
}
