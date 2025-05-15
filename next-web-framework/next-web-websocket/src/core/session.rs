use std::net::SocketAddr;

use axum::{
    extract::ws::{CloseFrame, Message},
    http::HeaderMap,
};
use flume::Sender;
use uuid::Uuid;

///
/// 表示一个 WebSocket 会话的结构体。
///
/// 该结构用于管理和标识单个 WebSocket 连接。
/// 包含会话 ID、消息发送通道和客户端远程地址。
///
/// # 字段说明
/// - `id`: 当前会话的唯一标识符（UUID）。
/// - `msg_channel`: 异步发送 WebSocket 消息的通道发送端。
/// - `remote_address`: 客户端的远程套接字地址。
///
/// Represents a WebSocket session.
///
/// This structure is used to manage and identify an individual WebSocket connection.
/// It contains the session ID, a channel sender for sending messages, and the remote address of the client.
///
/// # Fields
/// - `id`: Unique identifier for the session (UUID).
/// - `msg_channel`: Channel sender used to send WebSocket messages asynchronously.
/// - `remote_address`: The client's remote socket address.
///
#[derive(Clone)]
pub struct WebSocketSession {
    id: uuid::Uuid,
    msg_channel: Sender<Message>,
    remote_address: SocketAddr,
    header: HeaderMap,
    path: String,
}

impl WebSocketSession {
    ///
    /// 创建一个新的 WebSocket 会话，并生成唯一的会话 ID。
    ///
    /// # 参数
    /// - `msg_channel`: 用于向客户端发送消息的通道发送端。
    /// - `remote_address`: 客户端的远程地址。
    ///
    /// # 返回值
    /// 返回一个新的 `WebSocketSession` 实例。
    ///
    /// Creates a new WebSocket session with a unique ID.
    ///
    /// # Arguments
    /// - `msg_channel`: A sender channel used to send messages to the client.
    /// - `remote_address`: The client’s remote socket address.
    ///
    /// # Returns
    /// A new instance of `WebSocketSession`.
    ///
    pub fn new(
        msg_channel: Sender<Message>,
        remote_address: SocketAddr,
        header: HeaderMap,
        path: String,
    ) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            msg_channel,
            remote_address,
            header,
            path,
        }
    }

    ///
    /// 向当前 WebSocket 会话发送一条消息。
    ///
    /// # 参数
    /// - `message`: 要发送的 WebSocket 消息。
    ///
    /// # 返回值
    /// - 如果消息成功发送返回 `Ok(())`。
    /// - 如果发送失败（如通道关闭）返回 `Err(Message)`。
    ///
    /// Sends a message through the WebSocket session.
    ///
    /// # Arguments
    /// - `message`: The WebSocket message to be sent.
    ///
    /// # Returns
    /// - `Ok(())` if the message was successfully sent.
    /// - `Err(Message)` if the send operation failed (e.g., channel closed).
    ///
    pub async fn send_message(&self, message: Message) -> Result<(), Message> {
        if let Err(error) = self.msg_channel.send_async(message).await {
            return Err(error.0);
        }
        Ok(())
    }

    ///
    /// 判断当前 WebSocket 连接是否处于打开状态。
    ///
    /// 目前此方法始终返回 `false`，表示连接已关闭。
    /// 在实际实现中，可以检查底层连接状态。
    ///
    /// # 返回值
    /// 布尔值，表示当前会话是否处于活动状态。
    ///
    /// Checks whether the WebSocket connection is still open.
    ///
    /// Currently, this method always returns `false`, indicating that the connection is considered closed.
    /// In a real implementation, this could involve checking the state of the underlying connection.
    ///
    /// # Returns
    /// A boolean indicating whether the session is active/open.
    ///
    pub async fn is_open(&self) -> bool {
        self.msg_channel.is_disconnected()
    }

    ///
    /// 关闭当前 WebSocket 会话。
    ///
    /// 发送一个正常的关闭帧（code=1000, reason="0"），通知客户端服务器准备关闭连接。
    ///
    /// Closes the WebSocket session gracefully.
    ///
    /// Sends a close message with a normal closure code and an empty reason.
    /// This signals the client that the server intends to close the connection.
    ///
    pub async fn close(&self) {
        let _ = self
            .msg_channel
            .send_async(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: "0".into(),
            })))
            .await;
    }

    ///
    /// 获取当前会话的唯一标识符。
    ///
    /// # 返回值
    /// 返回对会话 UUID 的引用。
    ///
    /// Gets the unique identifier of the session.
    ///
    /// # Returns
    /// A reference to the session's UUID.
    ///
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    ///
    /// 获取当前连接客户端的远程地址。
    ///
    /// # 返回值
    /// 返回对客户端 `SocketAddr` 的引用。
    ///
    /// Gets the remote socket address of the connected client.
    ///
    /// # Returns
    /// A reference to the client's `SocketAddr`.
    ///
    pub fn remote_address(&self) -> &SocketAddr {
        &self.remote_address
    }

    ///
    /// 获取当前连接客户端的请求头部数据
    ///
    /// # 返回值
    /// 返回对 `HeaderMap` 的引用。
    ///
    /// Get the request header data of the currently connected client
    ///
    /// # Returns
    /// A reference to  `HeaderMap`.
    ///
    pub fn header(&self) -> &HeaderMap {
        &self.header
    }

    ///
    /// 获取当前连接客户端的请求路径
    ///
    /// # 返回值
    /// 返回对 `String` 的引用。
    ///
    /// Get the request path of the current connected client
    ///
    /// # Returns
    /// A reference to  `String`.
    ///
    pub fn path(&self) -> &str {
        self.path.as_str()
    }
}
