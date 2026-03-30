use chrono::{DateTime, Utc};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::mail::mime_message::MimeMessage;

/// Errors raised while building a MIME message.
#[derive(Debug, Error)]
pub enum MessagingError {
    #[error("invalid email address: {0}")]
    AddressError(String),
    #[error("multipart mode is required for inline resources or attachments")]
    MultipartRequired,
    #[error("message subject must not be empty")]
    EmptySubject,
    #[error("value must not be empty: {0}")]
    EmptyValue(&'static str),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Supported multipart strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MultipartMode {
    /// No multipart container.
    No,
    /// `multipart/mixed`
    Mixed,
    /// `multipart/related`
    Related,
    /// `multipart/mixed` with nested `multipart/related`
    #[default]
    MixedRelated,
}

/// Email address with an optional display name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternetAddress {
    address: String,
    personal: Option<String>,
}

impl InternetAddress {
    /// Create an address without a display name.
    pub fn new(address: impl Into<String>) -> Result<Self, MessagingError> {
        Self::with_personal(address, None::<String>)
    }

    /// Create an address with an optional display name.
    pub fn with_personal(
        address: impl Into<String>,
        personal: Option<impl Into<String>>,
    ) -> Result<Self, MessagingError> {
        let address = address.into().trim().to_string();
        validate_email(&address)?;

        Ok(Self {
            address,
            personal: personal.map(Into::into).filter(|value| !value.trim().is_empty()),
        })
    }

    /// Parse one or more comma-separated email addresses.
    pub fn parse(value: &str) -> Result<Vec<Self>, MessagingError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        trimmed
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(parse_mailbox_part)
            .collect()
    }

    /// Validate the underlying email address.
    pub fn validate(&self) -> Result<(), MessagingError> {
        validate_email(&self.address)
    }

    /// Return the raw email address.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Return the display name if present.
    pub fn personal(&self) -> Option<&str> {
        self.personal.as_deref()
    }

    fn formatted(&self) -> String {
        match &self.personal {
            Some(personal) => format!("{personal} <{}>", self.address),
            None => self.address.clone(),
        }
    }
}

impl fmt::Display for InternetAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.formatted())
    }
}

/// Byte source used for inline resources and attachments.
pub trait InputStreamSource {
    /// Read the full body into memory.
    fn read_bytes(&self) -> Result<Vec<u8>, io::Error>;

    /// Return a file name if one is naturally associated with the source.
    fn filename(&self) -> Option<&str> {
        None
    }
}

impl InputStreamSource for Vec<u8> {
    fn read_bytes(&self) -> Result<Vec<u8>, io::Error> {
        Ok(self.clone())
    }
}

impl InputStreamSource for &[u8] {
    fn read_bytes(&self) -> Result<Vec<u8>, io::Error> {
        Ok(self.to_vec())
    }
}

/// File-backed input stream source.
#[derive(Debug, Clone)]
pub struct FileResource {
    path: PathBuf,
}

impl FileResource {
    /// Create a new file resource.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Return the underlying path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl InputStreamSource for FileResource {
    fn read_bytes(&self) -> Result<Vec<u8>, io::Error> {
        fs::read(&self.path)
    }

    fn filename(&self) -> Option<&str> {
        self.path.file_name().and_then(|value| value.to_str())
    }
}

/// Memory-backed input stream source.
#[derive(Debug, Clone)]
pub struct BytesResource {
    bytes: Vec<u8>,
    filename: Option<String>,
}

impl BytesResource {
    /// Create a bytes resource with an optional logical filename.
    pub fn new(bytes: Vec<u8>, filename: Option<impl Into<String>>) -> Self {
        Self {
            bytes,
            filename: filename.map(Into::into),
        }
    }
}

impl InputStreamSource for BytesResource {
    fn read_bytes(&self) -> Result<Vec<u8>, io::Error> {
        Ok(self.bytes.clone())
    }

    fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }
}

/// Rust adaptation of Spring's `MimeMessageHelper`.
#[derive(Debug, Clone)]
pub struct MimeMessageHelper {
    message: MimeMessage,
    multipart_mode: MultipartMode,
    encoding: Option<String>,
    encode_filenames: bool,
    validate_addresses: bool,
}

impl MimeMessageHelper {
    /// Create a helper backed by a fresh message.
    pub fn new(multipart: bool, encoding: impl Into<Option<String>>) -> Self {
        Self::from_message(
            MimeMessage::new(),
            multipart,
            encoding.into().or_else(|| Some("UTF-8".to_string())),
        )
    }

    /// Create a helper from an existing message.
    pub fn from_message(
        message: MimeMessage,
        multipart: bool,
        encoding: impl Into<Option<String>>,
    ) -> Self {
        let multipart_mode = if multipart {
            MultipartMode::MixedRelated
        } else {
            MultipartMode::No
        };
        Self::from_message_with_mode(message, multipart_mode, encoding)
    }

    /// Create a helper using an explicit multipart mode.
    pub fn with_multipart_mode(
        multipart_mode: MultipartMode,
        encoding: impl Into<Option<String>>,
    ) -> Self {
        Self::from_message_with_mode(MimeMessage::new(), multipart_mode, encoding)
    }

    /// Create a helper from an existing message using an explicit multipart mode.
    pub fn from_message_with_mode(
        message: MimeMessage,
        multipart_mode: MultipartMode,
        encoding: impl Into<Option<String>>,
    ) -> Self {
        Self {
            message,
            multipart_mode,
            encoding: encoding.into().or_else(|| Some("UTF-8".to_string())),
            encode_filenames: false,
            validate_addresses: false,
        }
    }

    /// Return the underlying message.
    pub fn get_message(&self) -> &MimeMessage {
        &self.message
    }

    /// Return a mutable reference to the underlying message.
    pub fn get_message_mut(&mut self) -> &mut MimeMessage {
        &mut self.message
    }

    /// Consume the helper and return the underlying message.
    pub fn into_message(self) -> MimeMessage {
        self.message
    }

    /// Return the chosen multipart mode.
    pub fn multipart_mode(&self) -> MultipartMode {
        self.multipart_mode
    }

    /// Return whether multipart support is enabled.
    pub fn is_multipart(&self) -> bool {
        self.multipart_mode != MultipartMode::No
    }

    /// Return the configured encoding.
    pub fn get_encoding(&self) -> Option<&str> {
        self.encoding.as_deref()
    }

    /// Configure whether attachment file names should be encoded.
    pub fn set_encode_filenames(&mut self, encode_filenames: bool) {
        self.encode_filenames = encode_filenames;
    }

    /// Return whether attachment file names are encoded.
    pub fn is_encode_filenames(&self) -> bool {
        self.encode_filenames
    }

    /// Configure strict address validation.
    pub fn set_validate_addresses(&mut self, validate_addresses: bool) {
        self.validate_addresses = validate_addresses;
    }

    /// Return whether strict address validation is enabled.
    pub fn is_validate_addresses(&self) -> bool {
        self.validate_addresses
    }

    /// Set the `From` address.
    pub fn set_from(&mut self, from: impl AsRef<str>) -> Result<(), MessagingError> {
        let address = parse_single_address(from.as_ref())?;
        self.set_from_address(address)
    }

    /// Set the `From` address with a display name.
    pub fn set_from_with_personal(
        &mut self,
        from: impl AsRef<str>,
        personal: impl Into<String>,
    ) -> Result<(), MessagingError> {
        let address = InternetAddress::with_personal(from.as_ref(), Some(personal.into()))?;
        self.set_from_address(address)
    }

    /// Set the `From` address from a parsed address.
    pub fn set_from_address(&mut self, from: InternetAddress) -> Result<(), MessagingError> {
        self.validate_address(&from)?;
        self.message.set_from(from.to_string());
        Ok(())
    }

    /// Set the `Reply-To` address.
    pub fn set_reply_to(&mut self, reply_to: impl AsRef<str>) -> Result<(), MessagingError> {
        let address = parse_single_address(reply_to.as_ref())?;
        self.set_reply_to_address(address)
    }

    /// Set the `Reply-To` address with a display name.
    pub fn set_reply_to_with_personal(
        &mut self,
        reply_to: impl AsRef<str>,
        personal: impl Into<String>,
    ) -> Result<(), MessagingError> {
        let address = InternetAddress::with_personal(reply_to.as_ref(), Some(personal.into()))?;
        self.set_reply_to_address(address)
    }

    /// Set the `Reply-To` address from a parsed address.
    pub fn set_reply_to_address(
        &mut self,
        reply_to: InternetAddress,
    ) -> Result<(), MessagingError> {
        self.validate_address(&reply_to)?;
        self.message.set_reply_to(reply_to.to_string());
        Ok(())
    }

    /// Replace all `To` recipients.
    pub fn set_to<T, I>(&mut self, to: I) -> Result<(), MessagingError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        let addresses = self.collect_addresses(to)?;
        self.message
            .set_to(addresses.into_iter().map(|address| address.to_string()).collect());
        Ok(())
    }

    /// Replace all `Cc` recipients.
    pub fn set_cc<T, I>(&mut self, cc: I) -> Result<(), MessagingError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        let addresses = self.collect_addresses(cc)?;
        self.message
            .set_cc(addresses.into_iter().map(|address| address.to_string()).collect());
        Ok(())
    }

    /// Replace all `Bcc` recipients.
    pub fn set_bcc<T, I>(&mut self, bcc: I) -> Result<(), MessagingError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        let addresses = self.collect_addresses(bcc)?;
        self.message
            .set_bcc(addresses.into_iter().map(|address| address.to_string()).collect());
        Ok(())
    }

    /// Append one `To` recipient.
    pub fn add_to(&mut self, to: impl AsRef<str>) -> Result<(), MessagingError> {
        let address = parse_single_address(to.as_ref())?;
        self.validate_address(&address)?;
        self.message.add_to(address.to_string());
        Ok(())
    }

    /// Append one `Cc` recipient.
    pub fn add_cc(&mut self, cc: impl AsRef<str>) -> Result<(), MessagingError> {
        let address = parse_single_address(cc.as_ref())?;
        self.validate_address(&address)?;
        self.message.add_cc(address.to_string());
        Ok(())
    }

    /// Append one `Bcc` recipient.
    pub fn add_bcc(&mut self, bcc: impl AsRef<str>) -> Result<(), MessagingError> {
        let address = parse_single_address(bcc.as_ref())?;
        self.validate_address(&address)?;
        self.message.add_bcc(address.to_string());
        Ok(())
    }

    /// Set the message subject.
    pub fn set_subject(&mut self, subject: impl AsRef<str>) -> Result<(), MessagingError> {
        let subject = subject.as_ref().trim();
        if subject.is_empty() {
            return Err(MessagingError::EmptySubject);
        }
        self.message.set_subject(subject.to_string());
        Ok(())
    }

    /// Set the sent date.
    pub fn set_sent_date(&mut self, sent_date: DateTime<Utc>) -> Result<(), MessagingError> {
        self.message.set_sent_date(sent_date);
        Ok(())
    }

    /// Set a plain text or HTML body.
    pub fn set_text(
        &mut self,
        text: impl AsRef<str>,
        html: bool,
    ) -> Result<(), MessagingError> {
        let text = text.as_ref();
        if html {
            self.message.set_html(text.to_string());
        } else {
            self.message.set_text(text.to_string());
        }
        Ok(())
    }

    /// Set both plain text and HTML alternatives.
    pub fn set_texts(
        &mut self,
        plain_text: impl AsRef<str>,
        html_text: impl AsRef<str>,
    ) -> Result<(), MessagingError> {
        if !self.is_multipart() {
            return Err(MessagingError::MultipartRequired);
        }

        self.message.set_alternative(
            plain_text.as_ref().to_string(),
            html_text.as_ref().to_string(),
        );
        Ok(())
    }

    /// Add an inline resource.
    pub fn add_inline<S>(
        &mut self,
        content_id: impl AsRef<str>,
        source: S,
        content_type: impl AsRef<str>,
    ) -> Result<(), MessagingError>
    where
        S: InputStreamSource,
    {
        self.require_multipart()?;

        let content_id = content_id.as_ref().trim();
        if content_id.is_empty() {
            return Err(MessagingError::EmptyValue("content_id"));
        }

        let bytes = source.read_bytes()?;
        let name = source.filename().map(ToOwned::to_owned);
        let mime_type = normalize_content_type(content_type.as_ref(), source.filename());
        self.message
            .add_inline_resource(content_id, name, bytes, &mime_type);
        Ok(())
    }

    /// Add an inline resource from a file and infer its content type.
    pub fn add_inline_file(
        &mut self,
        content_id: impl AsRef<str>,
        path: impl Into<PathBuf>,
    ) -> Result<(), MessagingError> {
        let resource = FileResource::new(path.into());
        let content_type = guess_content_type(resource.filename());
        self.add_inline(content_id, resource, content_type)
    }

    /// Add an attachment.
    pub fn add_attachment<S>(
        &mut self,
        attachment_filename: impl AsRef<str>,
        source: S,
        content_type: impl AsRef<str>,
    ) -> Result<(), MessagingError>
    where
        S: InputStreamSource,
    {
        self.require_multipart()?;

        let attachment_filename = attachment_filename.as_ref().trim();
        if attachment_filename.is_empty() {
            return Err(MessagingError::EmptyValue("attachment_filename"));
        }

        let bytes = source.read_bytes()?;
        let filename = if self.encode_filenames {
            encode_filename(attachment_filename)
        } else {
            attachment_filename.to_string()
        };

        let mime_type = normalize_content_type(content_type.as_ref(), Some(attachment_filename));
        self.message.add_attachment(&filename, bytes, &mime_type);
        Ok(())
    }

    /// Add an attachment from a file and infer its content type.
    pub fn add_attachment_file(
        &mut self,
        attachment_filename: impl AsRef<str>,
        path: impl Into<PathBuf>,
    ) -> Result<(), MessagingError> {
        let resource = FileResource::new(path.into());
        let content_type = guess_content_type(Some(attachment_filename.as_ref()));
        self.add_attachment(attachment_filename, resource, content_type)
    }

    /// Set the priority header.
    pub fn set_priority(&mut self, priority: u8) -> Result<(), MessagingError> {
        if !(1..=5).contains(&priority) {
            return Err(MessagingError::EmptyValue("priority must be between 1 and 5"));
        }
        self.message
            .set_header("X-Priority", priority.to_string());
        Ok(())
    }

    fn collect_addresses<T, I>(&self, values: I) -> Result<Vec<InternetAddress>, MessagingError>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
    {
        values
            .into_iter()
            .map(Into::into)
            .map(|value: String| parse_single_address(&value))
            .map(|result| {
                let address = result?;
                self.validate_address(&address)?;
                Ok(address)
            })
            .collect()
    }

    fn validate_address(&self, address: &InternetAddress) -> Result<(), MessagingError> {
        if self.validate_addresses {
            address.validate()?;
        }
        Ok(())
    }

    fn require_multipart(&self) -> Result<(), MessagingError> {
        if self.is_multipart() {
            Ok(())
        } else {
            Err(MessagingError::MultipartRequired)
        }
    }
}

fn parse_single_address(value: &str) -> Result<InternetAddress, MessagingError> {
    let parsed = InternetAddress::parse(value)?;
    match parsed.as_slice() {
        [] => Err(MessagingError::AddressError(
            "address value must not be empty".to_string(),
        )),
        [single] => Ok(single.clone()),
        _ => Err(MessagingError::AddressError(format!(
            "expected a single address but got multiple: {value}"
        ))),
    }
}

fn parse_mailbox_part(value: &str) -> Result<InternetAddress, MessagingError> {
    let value = value.trim();

    if let Some((display_name, remainder)) = value.split_once('<') {
        let address = remainder
            .strip_suffix('>')
            .ok_or_else(|| MessagingError::AddressError(value.to_string()))?
            .trim();
        let display_name = display_name.trim().trim_matches('"');
        return InternetAddress::with_personal(
            address,
            (!display_name.is_empty()).then(|| display_name.to_string()),
        );
    }

    InternetAddress::new(value.to_string())
}

fn validate_email(address: &str) -> Result<(), MessagingError> {
    if address.is_empty() || address.contains(' ') || address.contains('<') || address.contains('>')
    {
        return Err(MessagingError::AddressError(address.to_string()));
    }

    let (local, domain) = address
        .split_once('@')
        .ok_or_else(|| MessagingError::AddressError(address.to_string()))?;

    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(MessagingError::AddressError(address.to_string()));
    }

    Ok(())
}

fn encode_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|ch| if ch.is_ascii_whitespace() { '_' } else { ch })
        .collect()
}

fn normalize_content_type(content_type: &str, fallback_filename: Option<&str>) -> String {
    let trimmed = content_type.trim();
    if trimmed.is_empty() {
        guess_content_type(fallback_filename)
    } else {
        trimmed.to_string()
    }
}

fn guess_content_type(filename: Option<&str>) -> String {
    let Some(filename) = filename else {
        return "application/octet-stream".to_string();
    };

    let extension = Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("txt") => "text/plain",
        Some("html") | Some("htm") => "text/html",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_address() {
        let mut helper = MimeMessageHelper::new(false, Some("UTF-8".to_string()));
        assert!(helper.set_from("invalid-email").is_err());
    }

    #[test]
    fn supports_plain_and_html_alternatives() {
        let mut helper = MimeMessageHelper::new(true, Some("UTF-8".to_string()));
        helper
            .set_texts("plain body", "<p>html body</p>")
            .expect("alternative text should be accepted");

        let message = helper.get_message();
        assert_eq!(message.text(), Some("plain body"));
        assert_eq!(message.html(), Some("<p>html body</p>"));
    }

    #[test]
    fn requires_multipart_for_attachments() {
        let mut helper = MimeMessageHelper::new(false, Some("UTF-8".to_string()));
        let result = helper.add_attachment("file.txt", b"content".as_slice(), "text/plain");
        assert!(matches!(result, Err(MessagingError::MultipartRequired)));
    }
}
