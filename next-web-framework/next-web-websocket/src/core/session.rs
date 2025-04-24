use std::net::SocketAddr;

use axum::extract::ws::{CloseFrame, Message};
use flume::Sender;

///
#[derive(Debug, Clone)]
pub struct WebSocketSession {
    msg_channel: Sender<Message>,
    remote_address: SocketAddr,
}

impl WebSocketSession {
    ///
    pub fn new(msg_channel: Sender<Message>, remote_address: SocketAddr) -> Self {
        Self {
            msg_channel,
            remote_address,
        }
    }

    ///
    pub async fn send_message(&self, message: Message) {
        let _ = self.msg_channel.send_async(message).await;
    }

    ///
    pub async fn is_open(&self) -> bool {
        false
    }

    /// 123
    pub async fn close(&self) {
        let _ = self.msg_channel
            .send_async(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: "Goodbye".into(),
            })))
            .await;
    }

    ///
    pub fn get_remote_address(&self) -> &SocketAddr {
        &self.remote_address
    }
}
