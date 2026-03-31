use chrono::{DateTime, Utc};

use crate::mail_error::MailError;
use crate::mail_message::MailMessage;
use crate::mime_message::MimeMessage;
use crate::mime_message_helper::MimeMessageHelper;

/// Adapter exposing a [`MailMessage`] view backed by a [`MimeMessageHelper`].
#[derive(Debug, Clone)]
pub struct MimeMailMessage {
    helper: MimeMessageHelper,
}

impl MimeMailMessage {
    /// Create from an existing helper.
    pub fn new(helper: MimeMessageHelper) -> Self {
        Self { helper }
    }

    /// Create from an existing helper.
    pub fn from_helper(helper: MimeMessageHelper) -> Self {
        Self::new(helper)
    }

    /// Create from an existing message.
    pub fn from_message(message: MimeMessage) -> Self {
        Self {
            helper: MimeMessageHelper::from_message(message, false, None::<String>),
        }
    }

    /// Return the underlying helper.
    pub fn helper(&self) -> &MimeMessageHelper {
        &self.helper
    }

    /// Return the underlying helper mutably.
    pub fn helper_mut(&mut self) -> &mut MimeMessageHelper {
        &mut self.helper
    }

    /// Consume `self` and return the helper.
    pub fn into_helper(self) -> MimeMessageHelper {
        self.helper
    }

    /// Return the underlying MIME message.
    pub fn message(&self) -> &MimeMessage {
        self.helper.get_message()
    }

    /// Consume `self` and return the MIME message.
    pub fn into_message(self) -> MimeMessage {
        self.helper.into_message()
    }

    /// Java-style alias for [`Self::helper`].
    pub fn get_mime_message_helper(&self) -> &MimeMessageHelper {
        self.helper()
    }

    /// Java-style alias for [`Self::message`].
    pub fn get_mime_message(&self) -> &MimeMessage {
        self.message()
    }

    /// Convenience method mirroring `DefaultMailMessage`.
    pub fn set_to_single(&mut self, to: impl Into<String>) -> Result<(), MailError> {
        self.set_to([to.into()])
    }

    /// Java-style single-recipient alias for `setTo(String)`.
    pub fn set_to_str(&mut self, to: impl Into<String>) -> Result<(), MailError> {
        self.set_to_single(to)
    }

    /// Convenience method mirroring `DefaultMailMessage`.
    pub fn set_cc_single(&mut self, cc: impl Into<String>) -> Result<(), MailError> {
        self.set_cc([cc.into()])
    }

    /// Java-style single-recipient alias for `setCc(String)`.
    pub fn set_cc_str(&mut self, cc: impl Into<String>) -> Result<(), MailError> {
        self.set_cc_single(cc)
    }

    /// Convenience method mirroring `DefaultMailMessage`.
    pub fn set_bcc_single(&mut self, bcc: impl Into<String>) -> Result<(), MailError> {
        self.set_bcc([bcc.into()])
    }

    /// Java-style single-recipient alias for `setBcc(String)`.
    pub fn set_bcc_str(&mut self, bcc: impl Into<String>) -> Result<(), MailError> {
        self.set_bcc_single(bcc)
    }

    /// Set HTML content.
    pub fn set_html_text(&mut self, html: impl Into<String>) -> Result<(), MailError> {
        let html = html.into();
        self.helper
            .set_text(&html, true)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    /// Set both plain text and HTML alternatives.
    pub fn set_alternative_texts(
        &mut self,
        plain_text: impl Into<String>,
        html_text: impl Into<String>,
    ) -> Result<(), MailError> {
        let plain_text = plain_text.into();
        let html_text = html_text.into();
        self.helper
            .set_texts(&plain_text, &html_text)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    /// Add a binary attachment.
    pub fn add_attachment(
        &mut self,
        name: impl AsRef<str>,
        content: Vec<u8>,
        mime_type: impl AsRef<str>,
    ) -> Result<(), MailError> {
        self.helper
            .add_attachment(name.as_ref(), content, mime_type.as_ref())
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    /// Add an inline resource from in-memory bytes with an explicit logical name.
    pub fn add_inline_named(
        &mut self,
        content_id: impl AsRef<str>,
        name: impl AsRef<str>,
        content: Vec<u8>,
        mime_type: impl AsRef<str>,
    ) -> Result<(), MailError> {
        self.helper
            .add_inline_named(
                content_id.as_ref(),
                name.as_ref(),
                content,
                mime_type.as_ref(),
            )
            .map_err(|error| MailError::ParseError(error.to_string()))
    }
}

impl MailMessage for MimeMailMessage {
    fn set_from<T>(&mut self, from: T) -> Result<(), MailError>
    where
        T: Into<String>,
    {
        let from = from.into();
        self.helper
            .set_from(&from)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn set_reply_to<T>(&mut self, reply_to: T) -> Result<(), MailError>
    where
        T: Into<String>,
    {
        let reply_to = reply_to.into();
        self.helper
            .set_reply_to(&reply_to)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn set_to<T, I>(&mut self, to: I) -> Result<(), MailError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        let to: Vec<String> = to.into_iter().map(Into::into).collect();
        self.helper
            .set_to(to)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn set_cc<T, I>(&mut self, cc: I) -> Result<(), MailError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        let cc: Vec<String> = cc.into_iter().map(Into::into).collect();
        self.helper
            .set_cc(cc)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn set_bcc<T, I>(&mut self, bcc: I) -> Result<(), MailError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        let bcc: Vec<String> = bcc.into_iter().map(Into::into).collect();
        self.helper
            .set_bcc(bcc)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn set_sent_date(&mut self, sent_date: DateTime<Utc>) -> Result<(), MailError> {
        self.helper
            .set_sent_date(sent_date)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn set_subject<T>(&mut self, subject: T) -> Result<(), MailError>
    where
        T: Into<String>,
    {
        let subject = subject.into();
        self.helper
            .set_subject(&subject)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn set_text<T>(&mut self, text: T) -> Result<(), MailError>
    where
        T: Into<String>,
    {
        let text = text.into();
        self.helper
            .set_text(&text, false)
            .map_err(|error| MailError::ParseError(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail_message::MailMessage;

    #[test]
    fn wraps_helper_and_message() {
        let helper = MimeMessageHelper::new(true, Some("UTF-8".to_string()));
        let mut mail_message = MimeMailMessage::from_helper(helper);

        mail_message
            .set_from("sender@example.com")
            .expect("from should be accepted");
        mail_message
            .set_to_single("recipient@example.com")
            .expect("recipient should be accepted");
        mail_message
            .set_subject("Test Subject")
            .expect("subject should be accepted");
        mail_message
            .set_text("Test content")
            .expect("text should be accepted");

        assert_eq!(mail_message.message().from(), Some("sender@example.com"));
        assert_eq!(mail_message.message().to(), &["recipient@example.com"]);
        assert_eq!(mail_message.message().subject(), Some("Test Subject"));
        assert_eq!(mail_message.message().text(), Some("Test content"));
    }

    #[test]
    fn rejects_invalid_input() {
        let helper = MimeMessageHelper::new(false, Some("UTF-8".to_string()));
        let mut mail_message = MimeMailMessage::from_helper(helper);

        assert!(mail_message.set_from("invalid-email").is_err());
        assert!(mail_message.set_subject("").is_err());
        assert!(
            mail_message
                .add_attachment("test.txt", vec![1, 2, 3], "text/plain")
                .is_err()
        );
    }

    #[test]
    fn supports_named_inline_resource() {
        let helper = MimeMessageHelper::new(true, Some("UTF-8".to_string()));
        let mut mail_message = MimeMailMessage::from_helper(helper);

        mail_message
            .add_inline_named("logo", "logo.png", vec![1, 2, 3], "image/png")
            .expect("named inline bytes should be accepted");

        let inline = &mail_message.message().inline_resources()[0];
        assert_eq!(inline.content_id, "logo");
        assert_eq!(inline.name.as_deref(), Some("logo.png"));
    }
}
