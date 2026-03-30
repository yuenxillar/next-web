use chrono::{DateTime, Utc};

use crate::mail::mail_error::MailError;

/// This is a common trait for mail messages, allowing a user to set key values required in assembling a
/// mail message, without needing to know if the underlying message is a simple text message or a more sophisticated MIME message.
/// Implemented by both SimpleMailMessage and MimeMessageHelper, to let message population code interact with a simple message
/// or a MIME message through a common trait.
pub trait MailMessage {
    /// Sets the sender (From) email address.
    fn set_from<T>(&mut self, from: T) -> Result<(), MailError>
    where
        T: Into<String>;

    // Sets the Reply-To email address.
    fn set_reply_to<T>(&mut self, reply_to: T) -> Result<(), MailError>
    where
        T: Into<String>;

    /// Sets the primary recipients (To) for the email.
    fn set_to<T, I>(&mut self, to: I) -> Result<(), MailError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>;

    /// Sets the carbon copy (Cc) recipients for the email.
    fn set_cc<T, I>(&mut self, cc: I) -> Result<(), MailError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>;

    /// Sets the blind carbon copy (Bcc) recipients for the email.
    fn set_bcc<T, I>(&mut self, bcc: I) -> Result<(), MailError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>;

    /// Sets the sent date of the email.
    fn set_sent_date(&mut self, sent_date: DateTime<Utc>) -> Result<(), MailError>;

    /// Sets the subject of the email.
    fn set_subject<T>(&mut self, subject: T) -> Result<(), MailError>
    where
        T: Into<String>;

    /// /// Sets the plain text content of the email.
    fn set_text<T>(&mut self, text: T) -> Result<(), MailError>
    where
        T: Into<String>;
}
