use std::error::Error;

use axum::extract::ws::{CloseFrame, Message};
use next_web_core::async_trait;

use super::session::WebSocketSession;

///
/// 自定义 WebSocket 操作结果类型。
///
/// 包装标准库的 `Result<T, E>`，使用 `Box<dyn Error + Send + Sync>` 作为错误类型，
/// 适用于异步处理和跨线程传递错误信息。
///
/// Custom result type for WebSocket operations.
///
/// Wraps the standard `Result<T, E>`, using `Box<dyn Error + Send + Sync>` as the error type,
/// suitable for asynchronous processing and cross-thread error propagation.
///
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

///
/// WebSocket 处理器接口。
///
/// 定义了 WebSocket 生命周期中的各个回调方法。
/// 所有方法都是异步的，并且要求实现者同时实现 `Send + Sync` 以支持多线程环境。
///
/// WebSocket handler trait.
///
/// Defines various callback methods during the WebSocket lifecycle.
/// All methods are asynchronous, and implementers must also implement `Send + Sync`
/// to support multi-threaded environments.
///
#[async_trait]
pub trait WebSocketHandler: Send + Sync {
    ///
    /// 返回当前处理器所绑定的 WebSocket 路径列表。
    ///
    /// 例如：`["/ws/chat", "/ws/notify"]`
    ///
    /// Returns a list of WebSocket paths that this handler is bound to.
    ///
    /// Example: `["/ws/chat", "/ws/notify"]`
    ///
    fn paths(&self) -> Vec<&'static str>;

    ///
    /// 当客户端建立 WebSocket 连接时调用的第一个方法。
    ///
    /// 可用于初始化会话、记录日志或分配资源。
    ///
    /// Called when a client establishes a WebSocket connection.
    ///
    /// Can be used for session initialization, logging, or resource allocation.
    ///
    async fn on_open(&self, session: &WebSocketSession) -> Result<()>;

    ///
    /// 当客户端发送消息到 WebSocket 时触发。
    ///
    /// 接收并处理文本或二进制消息。
    ///
    /// Triggered when a client sends a message over the WebSocket.
    ///
    /// Receives and processes text or binary messages.
    ///
    async fn on_message(&self, session: &WebSocketSession, message: Message) -> Result<()>;

    ///
    /// 当连接或消息传输过程中发生错误时调用。
    ///
    /// 可用于记录错误信息或进行异常恢复。
    ///
    /// Called when an error occurs during the connection or message transmission.
    ///
    /// Can be used for logging errors or performing exception recovery.
    ///
    async fn on_error(
        &self,
        session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> Result<()>;

    ///
    /// 在处理错误后或正常关闭连接时调用。
    ///
    /// 可用于释放资源、清理状态或记录断开连接事件。
    ///
    /// Called after handling an error or when the connection is closed normally.
    ///
    /// Can be used to release resources, clean up state, or log disconnection events.
    ///
    async fn on_close(&self, session: &WebSocketSession, close: Option<CloseFrame>) -> Result<()>;
}
