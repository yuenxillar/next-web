use next_web_core::async_trait;

use crate::{mail_error::MailError, mail_sender::MailSender, mime_message::MimeMessage};

#[async_trait]
pub trait MailService
where
    Self: Send + Sync,
    Self: 'static,
    Self: MailSender,
{
    /// Create a new JavaMail MimeMessage for the underlying JavaMail Session of this sender. Needs to
    /// be called to create MimeMessage instances that can be prepared by the client and passed to send(MimeMessage).
    fn create_mime_message(&self) -> MimeMessage;

    /// Send the given  MIME message. The message needs to have been created with create_mime_message().
    async fn send_mime_message(&self, mime_message: MimeMessage) -> Result<(), MailError>;

    /// Send the given array of MIME messages in batch.
    /// The messages need to have been created with create_mime_message().
    async fn send_mime_messages(&self, mime_messages: Vec<MimeMessage>) -> Result<(), MailError>;
}
