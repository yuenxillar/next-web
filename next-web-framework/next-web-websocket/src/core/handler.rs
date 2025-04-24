use std::error::Error;

use axum::extract::ws::{CloseFrame, Message};
use next_web_core::async_trait;

use super::session::WebSocketSession;

/// 
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// 
#[async_trait]
pub  trait WebSocketHandler: Send + Sync {
    /// 
    async fn on_open(&self, session: &WebSocketSession) -> Result<()>;

    ///
    async fn on_message(&self, session: &WebSocketSession, message: Message) -> Result<()>;

    ///
    async fn on_error(&self, session: &WebSocketSession, error: Box<dyn Error + Send + Sync>) -> Result<()>;

    ///
    async fn on_close(&self, session: &WebSocketSession, close: Option<CloseFrame>) -> Result<()>;
}