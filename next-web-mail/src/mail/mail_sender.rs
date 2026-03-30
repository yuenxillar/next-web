use next_web_core::async_trait;

use crate::mail::{default_mail_message::DefaultMailMessage, mail_error::MailError};

#[async_trait]
pub trait MailSender {
    /// Send the given  mail message.
    async fn send(&self, message: DefaultMailMessage) -> Result<(), MailError>;

    /// Send the given array of  mail messages in batch
    async fn send_batch(&self, messages: Vec<DefaultMailMessage>) -> Result<(), MailError>;
}
