use std::error::Error;

use axum::extract::ws::{CloseFrame, Message};
use next_web_core::async_trait;

use super::session::WebSocketSession;

///
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

///
#[async_trait]
pub trait WebSocketHandler: Send + Sync {
    ///
    fn paths(&self) -> Vec<&'static str>;

    /// When the socket connection enters, this method will be entered first
    async fn on_open(&self, session: &WebSocketSession) -> Result<()>;

    /// When the client sends a message, it will enter the following method
    async fn on_message(&self, session: &WebSocketSession, message: Message) -> Result<()>;

    /// When an error occurs during the connection process or message transmission, the following methods will be executed
    async fn on_error(
        &self,
        session: &WebSocketSession,
        error: Box<dyn Error + Send + Sync>,
    ) -> Result<()>;

    /// After handling the error, close the connection and proceed to the following method
    async fn on_close(&self, session: &WebSocketSession, close: Option<CloseFrame>) -> Result<()>;
}
