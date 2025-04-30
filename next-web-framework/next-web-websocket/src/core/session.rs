use std::net::SocketAddr;

use axum::extract::ws::{CloseFrame, Message};
use flume::Sender;
use uuid::Uuid;

///
#[derive(Debug, Clone)]
pub struct WebSocketSession {
    id: uuid::Uuid,
    msg_channel: Sender<Message>,
    remote_address: SocketAddr,
}

impl WebSocketSession {
    ///
    pub fn new(msg_channel: Sender<Message>, remote_address: SocketAddr) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            msg_channel,
            remote_address,
        }
    }

    ///
    pub async fn send_message(&self, message: Message) -> Result<(), Message> {
        if let Err(error) = self.msg_channel.send_async(message).await {
            return Err(error.0);
        }
        Ok(())
    }

    ///
    pub async fn is_open(&self) -> bool {
        false
    }

    /// 123
    pub async fn close(&self) {
        let _ = self
            .msg_channel
            .send_async(Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: "0".into(),
            })))
            .await;
    }

    /// Get session id
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    ///
    pub fn remote_address(&self) -> &SocketAddr {
        &self.remote_address
    }
    
}
