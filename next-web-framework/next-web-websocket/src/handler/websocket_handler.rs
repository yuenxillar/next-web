use std::error::Error;
use std::{net::SocketAddr, sync::Arc};

use axum::{body::Bytes, extract::ws::WebSocket, http::HeaderMap};
use futures::{stream::StreamExt, SinkExt};
use tracing::{debug, error, info};

use axum::extract::ws::{CloseFrame, Message};
use next_web_core::async_trait;

use crate::model::session::WebSocketSession;
use crate::model::ws_context::WebSocketContext;

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

/// ===============================================================

///
/// WebSocket连接处理主函数 - 每个连接都会生成一个实例
///
/// 负责处理WebSocket连接的整个生命周期，包括：
/// 1. 连接测试(Ping/Pong)
/// 2. 路径匹配处理器
/// 3. 消息收发处理
/// 4. 错误处理和连接关闭
///
/// Main function for handling WebSocket connections - one instance per connection
///
/// Handles the entire lifecycle of WebSocket connections including:
/// 1. Connection testing (Ping/Pong)
/// 2. Path matching handler
/// 3. Message sending/receiving
/// 4. Error handling and connection closure
///
/// # 参数/Parameters
/// - `socket`: WebSocket连接对象/WebSocket connection object
/// - `ctx`: WebSocket上下文，包含处理器注册信息/WebSocket context with handler registry
/// - `remote_address`: 客户端远程地址/Client remote address
/// - `path`: 请求路径/Request path
/// - `header`: HTTP请求头/HTTP request headers
pub(crate) async fn handle_socket(
    mut socket: WebSocket,
    ctx: Arc<WebSocketContext>,
    remote_address: SocketAddr,
    path: String,
    header: HeaderMap,
) {
    debug!("Start processing WebSocket connections: {remote_address}, Path: {path}");

    // 发送Ping包测试连接
    // 如果发送失败，说明连接可能已经断开
    //
    // Send a ping packet to test connection
    // If sending fails, the connection may have been disconnected
    if socket
        .send(Message::Ping(Bytes::from_static(&[1])))
        .await
        .is_err()
    {
        error!("Unable to send ping packet to client {remote_address}, The connection may have been disconnected");
        return;
    }

    // 通过路径匹配查找对应的处理器
    // 如果找不到匹配的处理器，则关闭连接
    //
    // Match the corresponding handler through path matching
    // If no matching handler is found, close the connection
    let handler = match ctx.get_handler(&path) {
        Some(handler) => handler,
        None => {
            error!("Path not found {path} The corresponding processor's connection will be closed");
            let _ = socket.close().await;
            return;
        }
    };

    let (msg_sender, msg_receiver) = flume::unbounded();
    let session = WebSocketSession::new(msg_sender, remote_address, header, path.to_owned());

    // 分离socket实现同时收发
    // 使用flume通道实现异步消息传递
    // 在此示例中，我们将基于服务器内部事件(如定时器)向客户端发送主动消息
    //
    // Split socket for simultaneous send/receive
    // Using flume channel for async message passing
    // In this example we'll send unsolicited messages based on server events (e.g. timer)
    let (mut stream_sender, mut stream_receiver) = socket.split();

    // 处理连接打开事件
    // 调用处理器的on_open方法进行初始化
    //
    // Handle on_open event
    // Call handler's on_open method for initialization
    if let Err(e) = handler.on_open(&session).await {
        error!("Event on_open processing failed: {e}, Client: {remote_address}, Path: {path}");
        return;
    }

    info!("WS Connection established: {remote_address}, Path: {path}");

    // 发送消息到客户端
    // 使用tokio::spawn创建异步任务处理消息发送
    // 如果收到Close消息则终止循环
    //
    // Send messages to client
    // Use tokio::spawn to create async task for message sending
    // Terminate loop if Close message is received
    tokio::spawn(async move {
        while let Ok(msg) = msg_receiver.recv_async().await {
            let close = if let Message::Close(_) = &msg {
                true
            } else {
                false
            };
            if let Err(e) = stream_sender.send(msg).await {
                error!("Sending message to client failed: {e}, Client: {remote_address}");
                break;
            } else {
                if close {
                    break;
                }
            }
        }

        drop(msg_receiver);
    });

    // 接收客户端消息
    // 循环接收并处理客户端发送的消息
    // 如果处理失败则调用on_error方法
    //
    // Receive client messages
    // Loop to receive and process messages from client
    // Call on_error method if processing fails
    while let Some(result) = stream_receiver.next().await {
        match result {
            Ok(msg) => {
                if let Err(e) = handler.on_message(&session, msg).await {
                    error!("Failed to process client message: {e}, Client: {remote_address}");
                    if let Err(e) = handler.on_error(&session, e).await {
                        error!("Failed to handle error event: {e}, Client: {remote_address}");
                    }
                    break;
                }
            }

            Err(e) => {
                error!("Failed to receive client message: {e}, Client: {remote_address}");
                let boxed_err = Box::new(e);
                let _ = handler.on_error(&session, boxed_err).await;
                break;
            }
        }
    }

    // 连接关闭处理
    // 调用处理器的on_close方法进行资源清理
    //
    // Handle connection closure
    // Call handler's on_close method for resource cleanup
    if let Err(e) = handler.on_close(&session, None).await {
        error!("Failed to handle connection closure event: {e}, Client: {remote_address}");
    }
}
