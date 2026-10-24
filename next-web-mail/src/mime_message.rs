use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Attachment stored on a MIME message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// Attachment file name exposed to the recipient.
    pub name: String,
    /// Attachment bytes.
    pub content: Vec<u8>,
    /// Attachment content type.
    pub mime_type: String,
}

/// Inline resource embedded into the HTML body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineResource {
    /// Content id used from HTML, without the surrounding `cid:` prefix.
    pub content_id: String,
    /// Optional resource name.
    pub name: Option<String>,
    /// Inline resource bytes.
    pub content: Vec<u8>,
    /// Resource content type.
    pub mime_type: String,
}

/// In-memory MIME message model used by the mail helper.
#[derive(Debug, Clone, Default)]
pub struct MimeMessage {
    from: Option<String>,
    reply_to: Option<String>,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: Option<String>,
    text: Option<String>,
    html: Option<String>,
    sent_date: Option<DateTime<Utc>>,
    attachments: Vec<Attachment>,
    inline_resources: Vec<InlineResource>,
    headers: HashMap<String, String>,
}

impl MimeMessage {
    /// Create an empty MIME message.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the sender address.
    pub fn set_from(&mut self, from: impl Into<String>) {
        self.from = Some(from.into());
    }

    /// Set the reply-to address.
    pub fn set_reply_to(&mut self, reply_to: impl Into<String>) {
        self.reply_to = Some(reply_to.into());
    }

    /// Replace all `To` recipients.
    pub fn set_to(&mut self, to: Vec<String>) {
        self.to = to;
    }

    /// Append one `To` recipient.
    pub fn add_to(&mut self, to: impl Into<String>) {
        self.to.push(to.into());
    }

    /// Replace all `Cc` recipients.
    pub fn set_cc(&mut self, cc: Vec<String>) {
        self.cc = cc;
    }

    /// Append one `Cc` recipient.
    pub fn add_cc(&mut self, cc: impl Into<String>) {
        self.cc.push(cc.into());
    }

    /// Replace all `Bcc` recipients.
    pub fn set_bcc(&mut self, bcc: Vec<String>) {
        self.bcc = bcc;
    }

    /// Append one `Bcc` recipient.
    pub fn add_bcc(&mut self, bcc: impl Into<String>) {
        self.bcc.push(bcc.into());
    }

    /// Set the message subject.
    pub fn set_subject(&mut self, subject: impl Into<String>) {
        self.subject = Some(subject.into());
    }

    /// Set a plain text body and clear any previous HTML-only body.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = Some(text.into());
        self.html = None;
    }

    /// Set an HTML body and clear any previous plain-text-only body.
    pub fn set_html(&mut self, html: impl Into<String>) {
        self.html = Some(html.into());
        self.text = None;
    }

    /// Set both plain text and HTML alternatives.
    pub fn set_alternative(&mut self, text: impl Into<String>, html: impl Into<String>) {
        self.text = Some(text.into());
        self.html = Some(html.into());
    }

    /// Set the sent date.
    pub fn set_sent_date(&mut self, sent_date: DateTime<Utc>) {
        self.sent_date = Some(sent_date);
    }

    /// Add an attachment.
    pub fn add_attachment(&mut self, name: &str, content: Vec<u8>, mime_type: &str) {
        self.attachments.push(Attachment {
            name: name.to_string(),
            content,
            mime_type: mime_type.to_string(),
        });
    }

    /// Add an inline resource.
    pub fn add_inline_resource(
        &mut self,
        content_id: &str,
        name: Option<String>,
        content: Vec<u8>,
        mime_type: &str,
    ) {
        self.inline_resources.push(InlineResource {
            content_id: content_id.to_string(),
            name,
            content,
            mime_type: mime_type.to_string(),
        });
    }

    /// Set or replace a message header.
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    /// Return the sender address.
    pub fn from(&self) -> Option<&str> {
        self.from.as_deref()
    }

    /// Return the reply-to address.
    pub fn reply_to(&self) -> Option<&str> {
        self.reply_to.as_deref()
    }

    /// Return all `To` recipients.
    pub fn to(&self) -> &[String] {
        &self.to
    }

    /// Return all `Cc` recipients.
    pub fn cc(&self) -> &[String] {
        &self.cc
    }

    /// Return all `Bcc` recipients.
    pub fn bcc(&self) -> &[String] {
        &self.bcc
    }

    /// Return the message subject.
    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    /// Return the plain text body if present.
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Return the HTML body if present.
    pub fn html(&self) -> Option<&str> {
        self.html.as_deref()
    }

    /// Return the sent date.
    pub fn sent_date(&self) -> Option<DateTime<Utc>> {
        self.sent_date
    }

    /// Return all attachments.
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }

    /// Return all inline resources.
    pub fn inline_resources(&self) -> &[InlineResource] {
        &self.inline_resources
    }

    /// Return all message headers.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Return a specific header if present.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}
