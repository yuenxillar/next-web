use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Default mail message that encapsulates basic email information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultMailMessage {
    /// Sender's email address
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,

    /// Reply-to email address
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,

    /// Primary recipients
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<Vec<String>>,

    /// Carbon copy recipients
    #[serde(skip_serializing_if = "Option::is_none")]
    cc: Option<Vec<String>>,

    /// Blind carbon copy recipients
    #[serde(skip_serializing_if = "Option::is_none")]
    bcc: Option<Vec<String>>,

    /// Date when the message was sent
    #[serde(skip_serializing_if = "Option::is_none")]
    sent_date: Option<DateTime<Utc>>,

    /// Email subject
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,

    /// Email body text
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

impl DefaultMailMessage {
    /// Creates a new empty mail message
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new mail message by copying from another instance
    pub fn from_original(original: &DefaultMailMessage) -> Self {
        Self {
            from: original.from.clone(),
            reply_to: original.reply_to.clone(),
            to: original.to.clone(),
            cc: original.cc.clone(),
            bcc: original.bcc.clone(),
            sent_date: original.sent_date,
            subject: original.subject.clone(),
            text: original.text.clone(),
        }
    }

    pub fn from(&self) -> Option<&str> {
        self.from.as_deref()
    }

    pub fn reply_to(&self) -> Option<&str> {
        self.reply_to.as_deref()
    }

    pub fn to(&self) -> Option<&[String]> {
        self.to.as_deref()
    }

    pub fn cc(&self) -> Option<&[String]> {
        self.cc.as_deref()
    }

    pub fn bcc(&self) -> Option<&[String]> {
        self.bcc.as_deref()
    }

    pub fn sent_date(&self) -> Option<DateTime<Utc>> {
        self.sent_date
    }

    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    // Setters
    pub fn set_from(&mut self, from: impl Into<String>) -> &mut Self {
        self.from = Some(from.into());
        self
    }

    pub fn set_reply_to(&mut self, reply_to: impl Into<String>) -> &mut Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    pub fn set_to_single(&mut self, to: impl Into<String>) -> &mut Self {
        self.to = Some(vec![to.into()]);
        self
    }

    pub fn set_to(&mut self, to: Vec<String>) -> &mut Self {
        self.to = Some(to);
        self
    }

    pub fn set_cc_single(&mut self, cc: impl Into<String>) -> &mut Self {
        self.cc = Some(vec![cc.into()]);
        self
    }

    pub fn set_cc(&mut self, cc: Vec<String>) -> &mut Self {
        self.cc = Some(cc);
        self
    }

    pub fn set_bcc_single(&mut self, bcc: impl Into<String>) -> &mut Self {
        self.bcc = Some(vec![bcc.into()]);
        self
    }

    pub fn set_bcc(&mut self, bcc: Vec<String>) -> &mut Self {
        self.bcc = Some(bcc);
        self
    }

    pub fn set_sent_date(&mut self, sent_date: DateTime<Utc>) -> &mut Self {
        self.sent_date = Some(sent_date);
        self
    }

    pub fn set_subject(&mut self, subject: impl Into<String>) -> &mut Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.text = Some(text.into());
        self
    }

    /// Copies all non-null fields from this message to a target message
    pub fn copy_to(&self, target: &mut DefaultMailMessage) {
        if let Some(ref from) = self.from {
            target.from = Some(from.clone());
        }
        if let Some(ref reply_to) = self.reply_to {
            target.reply_to = Some(reply_to.clone());
        }
        if let Some(ref to) = self.to {
            target.to = Some(to.clone());
        }
        if let Some(ref cc) = self.cc {
            target.cc = Some(cc.clone());
        }
        if let Some(ref bcc) = self.bcc {
            target.bcc = Some(bcc.clone());
        }
        if let Some(sent_date) = self.sent_date {
            target.sent_date = Some(sent_date);
        }
        if let Some(ref subject) = self.subject {
            target.subject = Some(subject.clone());
        }
        if let Some(ref text) = self.text {
            target.text = Some(text.clone());
        }
    }

    /// Merges another message into this one (non-overwriting)
    pub fn merge(&mut self, other: &DefaultMailMessage) {
        if self.from.is_none() {
            self.from = other.from.clone();
        }
        if self.reply_to.is_none() {
            self.reply_to = other.reply_to.clone();
        }
        if self.to.is_none() {
            self.to = other.to.clone();
        }
        if self.cc.is_none() {
            self.cc = other.cc.clone();
        }
        if self.bcc.is_none() {
            self.bcc = other.bcc.clone();
        }
        if self.sent_date.is_none() {
            self.sent_date = other.sent_date;
        }
        if self.subject.is_none() {
            self.subject = other.subject.clone();
        }
        if self.text.is_none() {
            self.text = other.text.clone();
        }
    }

    /// Validates that the message has the minimum required fields
    pub fn validate(&self) -> Result<(), MailMessageError> {
        if self.to.is_none() && self.cc.is_none() && self.bcc.is_none() {
            return Err(MailMessageError::NoRecipients);
        }
        if self.subject.is_none() && self.text.is_none() {
            return Err(MailMessageError::NoContent);
        }
        Ok(())
    }
}

impl PartialEq for DefaultMailMessage {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from
            && self.reply_to == other.reply_to
            && self.to == other.to
            && self.cc == other.cc
            && self.bcc == other.bcc
            && self.sent_date == other.sent_date
            && self.subject == other.subject
            && self.text == other.text
    }
}

impl Eq for DefaultMailMessage {}

impl fmt::Display for DefaultMailMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DefaultMailMessage: ")?;
        if let Some(ref from) = self.from {
            write!(f, "from={}; ", from)?;
        }
        if let Some(ref reply_to) = self.reply_to {
            write!(f, "reply_to={}; ", reply_to)?;
        }
        if let Some(ref to) = self.to {
            write!(f, "to={}; ", to.join(", "))?;
        }
        if let Some(ref cc) = self.cc {
            write!(f, "cc={}; ", cc.join(", "))?;
        }
        if let Some(ref bcc) = self.bcc {
            write!(f, "bcc={}; ", bcc.join(", "))?;
        }
        if let Some(sent_date) = self.sent_date {
            write!(f, "sent_date={}; ", sent_date)?;
        }
        if let Some(ref subject) = self.subject {
            write!(f, "subject={}; ", subject)?;
        }
        if let Some(ref text) = self.text {
            write!(f, "text={}", text)?;
        }
        Ok(())
    }
}

/// Error types for mail message validation
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MailMessageError {
    #[error("No recipients specified (to, cc, or bcc must be provided)")]
    NoRecipients,

    #[error("No content specified (subject or text must be provided)")]
    NoContent,
}

/// Builder pattern for constructing mail messages
#[derive(Debug, Clone, Default)]
pub struct DefaultMailMessageBuilder {
    message: DefaultMailMessage,
}

impl DefaultMailMessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.message.from = Some(from.into());
        self
    }

    pub fn reply_to(mut self, reply_to: impl Into<String>) -> Self {
        self.message.reply_to = Some(reply_to.into());
        self
    }

    pub fn to(mut self, to: impl Into<String>) -> Self {
        let to_vec = match self.message.to {
            Some(mut existing) => {
                existing.push(to.into());
                existing
            }
            None => vec![to.into()],
        };
        self.message.to = Some(to_vec);
        self
    }

    pub fn to_multiple(mut self, to: Vec<String>) -> Self {
        self.message.to = Some(to);
        self
    }

    pub fn cc(mut self, cc: impl Into<String>) -> Self {
        let cc_vec = match self.message.cc {
            Some(mut existing) => {
                existing.push(cc.into());
                existing
            }
            None => vec![cc.into()],
        };
        self.message.cc = Some(cc_vec);
        self
    }

    pub fn bcc(mut self, bcc: impl Into<String>) -> Self {
        let bcc_vec = match self.message.bcc {
            Some(mut existing) => {
                existing.push(bcc.into());
                existing
            }
            None => vec![bcc.into()],
        };
        self.message.bcc = Some(bcc_vec);
        self
    }

    pub fn sent_date(mut self, sent_date: DateTime<Utc>) -> Self {
        self.message.sent_date = Some(sent_date);
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.message.subject = Some(subject.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.message.text = Some(text.into());
        self
    }

    pub fn build(self) -> Result<DefaultMailMessage, MailMessageError> {
        self.message.validate()?;
        Ok(self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_create_message() {
        let msg = DefaultMailMessage::new()
            .set_from("sender@example.com")
            .set_to_single("recipient@example.com")
            .set_subject("Test Subject")
            .set_text("Test content")
            .clone();

        assert_eq!(msg.from(), Some("sender@example.com"));
        assert_eq!(msg.to().unwrap()[0], "recipient@example.com");
        assert_eq!(msg.subject(), Some("Test Subject"));
        assert_eq!(msg.text(), Some("Test content"));
    }

    #[test]
    fn test_builder_pattern() {
        let sent_date = Utc::now();

        let msg = DefaultMailMessageBuilder::new()
            .from("sender@example.com")
            .to("recipient1@example.com")
            .to("recipient2@example.com")
            .cc("cc@example.com")
            .subject("Builder Test")
            .text("Builder content")
            .sent_date(sent_date)
            .build()
            .unwrap();

        assert_eq!(msg.from(), Some("sender@example.com"));
        assert_eq!(msg.to().unwrap().len(), 2);
        assert_eq!(msg.cc().unwrap()[0], "cc@example.com");
        assert_eq!(msg.sent_date(), Some(sent_date));
    }

    #[test]
    fn test_copy_to() {
        let mut original = DefaultMailMessage::new();
        original
            .set_from("sender@example.com")
            .set_to_single("to@example.com")
            .set_subject("Subject");

        let mut target = DefaultMailMessage::new();
        original.copy_to(&mut target);

        assert_eq!(target.from(), original.from());
        assert_eq!(target.to(), original.to());
        assert_eq!(target.subject(), original.subject());
    }

    #[test]
    fn test_validation() {
        let mut msg = DefaultMailMessage::new();

        // Missing recipients
        assert!(msg.validate().is_err());

        msg.set_to_single("test@example.com");

        // Missing content
        assert!(msg.validate().is_err());

        msg.set_subject("Subject");
        assert!(msg.validate().is_ok());
    }

    #[test]
    fn test_equality() {
        let mut msg1 = DefaultMailMessage::new();
        msg1.set_from("sender@example.com").set_subject("Test");

        let mut msg2 = DefaultMailMessage::new();
        msg2.set_from("sender@example.com").set_subject("Test");

        assert_eq!(msg1, msg2);

        msg2.set_text("Different");
        assert_ne!(msg1, msg2);
    }

    #[test]
    fn test_display() {
        let mut msg = DefaultMailMessage::new();
        msg.set_from("sender@example.com")
            .set_to_single("to@example.com")
            .set_subject("Hello")
            .set_text("World");

        let display_str = format!("{}", msg);
        assert!(display_str.contains("from=sender@example.com"));
        assert!(display_str.contains("to=to@example.com"));
        assert!(display_str.contains("subject=Hello"));
        assert!(display_str.contains("text=World"));
    }
}
