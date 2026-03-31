use std::{
    collections::HashMap,
    ops::Deref,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{
        Attachment as LettreAttachment, Mailbox, MessageBuilder, MultiPart, SinglePart,
        header::{ContentType, HeaderName, HeaderValue},
    },
    transport::smtp::authentication::Credentials,
};
use next_web_core::{
    async_trait, error::BoxError, impl_service,
    mime_type::configurable_mime_file_type_map::ConfigurableMimeFileTypeMap,
};

use crate::{
    autoconfigure::mail_properties::MailProperties,
    default_mail_message::DefaultMailMessage,
    mail_error::MailError,
    mail_sender::MailSender,
    mail_service::MailService,
    mime_message::{Attachment, InlineResource, MimeMessage},
};

#[derive(Debug)]
enum BuiltBody {
    Single(SinglePart),
    Multi(MultiPart),
}

/// Default mail service backed by SMTP.
#[derive(Clone)]
pub struct DefaultMailService {
    /// Mail properties.
    mail_properties: MailProperties,

    /// Session-style properties.
    properties: Option<HashMap<String, String>>,

    /// SMTP transport.
    smtp_transport: AsyncSmtpTransport<Tokio1Executor>,

    /// Default MIME type map.
    default_file_type_map: ConfigurableMimeFileTypeMap,
}

impl DefaultMailService {
    /// Create a new mail service instance.
    pub fn new(mut mail_properties: MailProperties) -> Result<Self, BoxError> {
        let MailProperties {
            host,
            port,
            username,
            password,
            protocol: _,
            default_encoding: _,
            properties,
            ssl,
        } = &mail_properties;

        let transport = if ssl.enabled() {
            AsyncSmtpTransport::<Tokio1Executor>::relay(host.as_deref().unwrap_or_default())?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(
                host.as_deref().unwrap_or_default(),
            )
        };

        let smtp_transport = transport
            .port(*port)
            .credentials(Credentials::new(
                username.clone().unwrap_or_default(),
                password.clone().unwrap_or_default(),
            ))
            .build();

        let mut default_file_type_map = ConfigurableMimeFileTypeMap::default();
        default_file_type_map.load_file("", None)?;

        let properties = properties
            .clone()
            .or_else(|| mail_properties.properties.take());

        Ok(Self {
            mail_properties,
            properties,
            smtp_transport,
            default_file_type_map,
        })
    }

    async fn do_send<T>(&self, messages: T) -> Result<(), MailError>
    where
        T: IntoIterator<Item = Message>,
    {
        let mut failures = Vec::new();

        for message in messages {
            if let Err(error) = self.smtp_transport.send(message).await {
                failures.push(Self::map_transport_error(error));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(MailError::SendError(
                failures
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            ))
        }
    }

    fn simple_message_to_lettre(&self, message: DefaultMailMessage) -> Result<Message, MailError> {
        message
            .validate()
            .map_err(|error| MailError::ParseError(error.to_string()))?;

        let empty: Vec<String> = Vec::new();
        let mut builder = self.prepare_builder(
            message.from(),
            message.reply_to(),
            message.to().unwrap_or(&empty),
            message.cc().unwrap_or(&empty),
            message.bcc().unwrap_or(&empty),
            message.subject(),
            message.sent_date(),
            None,
        )?;

        builder = builder.header(self.text_content_type(false));

        builder
            .body(message.text().unwrap_or_default().to_string())
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn mime_message_to_lettre(&self, message: MimeMessage) -> Result<Message, MailError> {
        self.validate_mime_message(&message)?;

        let builder = self.prepare_builder(
            message.from(),
            message.reply_to(),
            message.to(),
            message.cc(),
            message.bcc(),
            message.subject(),
            message.sent_date(),
            Some(message.headers()),
        )?;

        match self.build_mime_body(&message)? {
            BuiltBody::Single(single_part) => builder
                .singlepart(single_part)
                .map_err(|error| MailError::ParseError(error.to_string())),
            BuiltBody::Multi(multi_part) => builder
                .multipart(multi_part)
                .map_err(|error| MailError::ParseError(error.to_string())),
        }
    }

    fn prepare_builder(
        &self,
        from: Option<&str>,
        reply_to: Option<&str>,
        to: &[String],
        cc: &[String],
        bcc: &[String],
        subject: Option<&str>,
        sent_date: Option<chrono::DateTime<chrono::Utc>>,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<MessageBuilder, MailError> {
        if to.is_empty() && cc.is_empty() && bcc.is_empty() {
            return Err(MailError::ParseError(
                "mail recipients must not be empty".to_string(),
            ));
        }

        let mut builder = Message::builder().from(self.resolve_from_mailbox(from)?);

        if let Some(reply_to) = reply_to {
            builder = builder.reply_to(self.parse_mailbox(reply_to)?);
        }

        for recipient in to {
            builder = builder.to(self.parse_mailbox(recipient)?);
        }

        for recipient in cc {
            builder = builder.cc(self.parse_mailbox(recipient)?);
        }

        for recipient in bcc {
            builder = builder.bcc(self.parse_mailbox(recipient)?);
        }

        if let Some(subject) = subject {
            builder = builder.subject(subject.to_string());
        }

        if let Some(sent_date) = sent_date {
            builder = builder.date(Self::chrono_to_system_time(sent_date)?);
        }

        if let Some(headers) = headers {
            for (name, value) in headers {
                if Self::is_reserved_header(name) {
                    continue;
                }

                builder = builder.raw_header(HeaderValue::new(
                    HeaderName::new_from_ascii(name.clone())
                        .map_err(|error| MailError::ParseError(error.to_string()))?,
                    value.clone(),
                ));
            }
        }

        Ok(builder)
    }

    fn build_mime_body(&self, message: &MimeMessage) -> Result<BuiltBody, MailError> {
        let text_part = message
            .text()
            .map(|text| self.build_text_part(text, false))
            .transpose()?;
        let html_part = message
            .html()
            .map(|html| self.build_text_part(html, true))
            .transpose()?;
        let inline_parts = message
            .inline_resources()
            .iter()
            .map(|resource| self.build_inline_part(resource))
            .collect::<Result<Vec<_>, _>>()?;
        let attachment_parts = message
            .attachments()
            .iter()
            .map(|attachment| self.build_attachment_part(attachment))
            .collect::<Result<Vec<_>, _>>()?;

        self.build_body_part(text_part, html_part, inline_parts, attachment_parts)
    }

    fn build_body_part(
        &self,
        text_part: Option<SinglePart>,
        html_part: Option<SinglePart>,
        inline_parts: Vec<SinglePart>,
        attachment_parts: Vec<SinglePart>,
    ) -> Result<BuiltBody, MailError> {
        let base_body = self.build_content_part(text_part, html_part, inline_parts)?;

        if attachment_parts.is_empty() {
            return base_body
                .ok_or_else(|| MailError::ParseError("mime message has no content".to_string()));
        }

        let mut mixed = MultiPart::mixed().build();

        if let Some(base_body) = base_body {
            mixed = match base_body {
                BuiltBody::Single(single_part) => mixed.singlepart(single_part),
                BuiltBody::Multi(multi_part) => mixed.multipart(multi_part),
            };
        }

        for attachment in attachment_parts {
            mixed = mixed.singlepart(attachment);
        }

        Ok(BuiltBody::Multi(mixed))
    }

    fn build_content_part(
        &self,
        text_part: Option<SinglePart>,
        html_part: Option<SinglePart>,
        inline_parts: Vec<SinglePart>,
    ) -> Result<Option<BuiltBody>, MailError> {
        let body = match (text_part, html_part, inline_parts.is_empty()) {
            (Some(text_part), Some(html_part), true) => Some(BuiltBody::Multi(
                MultiPart::alternative()
                    .singlepart(text_part)
                    .singlepart(html_part),
            )),
            (Some(text_part), Some(html_part), false) => {
                let related = inline_parts.into_iter().fold(
                    MultiPart::related().singlepart(html_part),
                    |multipart, inline| multipart.singlepart(inline),
                );

                Some(BuiltBody::Multi(
                    MultiPart::alternative()
                        .singlepart(text_part)
                        .multipart(related),
                ))
            }
            (Some(text_part), None, true) => Some(BuiltBody::Single(text_part)),
            (Some(text_part), None, false) => {
                Some(BuiltBody::Multi(inline_parts.into_iter().fold(
                    MultiPart::related().singlepart(text_part),
                    |multipart, inline| multipart.singlepart(inline),
                )))
            }
            (None, Some(html_part), true) => Some(BuiltBody::Single(html_part)),
            (None, Some(html_part), false) => {
                Some(BuiltBody::Multi(inline_parts.into_iter().fold(
                    MultiPart::related().singlepart(html_part),
                    |multipart, inline| multipart.singlepart(inline),
                )))
            }
            (None, None, false) => None,
            (None, None, true) => None,
        };

        Ok(body)
    }

    fn build_text_part(&self, body: &str, html: bool) -> Result<SinglePart, MailError> {
        Ok(SinglePart::builder()
            .header(self.text_content_type(html))
            .body(body.to_string()))
    }

    fn build_attachment_part(&self, attachment: &Attachment) -> Result<SinglePart, MailError> {
        Ok(LettreAttachment::new(attachment.name.clone()).body(
            attachment.content.clone(),
            self.parse_content_type(
                &attachment.mime_type,
                self.lookup_mime_type(&attachment.name, "application/octet-stream"),
            )?,
        ))
    }

    fn build_inline_part(&self, resource: &InlineResource) -> Result<SinglePart, MailError> {
        let attachment = match &resource.name {
            Some(name) => {
                LettreAttachment::new_inline_with_name(resource.content_id.clone(), name.clone())
            }
            None => LettreAttachment::new_inline(resource.content_id.clone()),
        };

        let fallback_name = resource.name.as_deref().unwrap_or(&resource.content_id);
        Ok(attachment.body(
            resource.content.clone(),
            self.parse_content_type(
                &resource.mime_type,
                self.lookup_mime_type(fallback_name, "application/octet-stream"),
            )?,
        ))
    }

    fn validate_mime_message(&self, message: &MimeMessage) -> Result<(), MailError> {
        if message.to().is_empty() && message.cc().is_empty() && message.bcc().is_empty() {
            return Err(MailError::ParseError(
                "mime message recipients must not be empty".to_string(),
            ));
        }

        if message.subject().is_none()
            && message.text().is_none()
            && message.html().is_none()
            && message.attachments().is_empty()
            && message.inline_resources().is_empty()
        {
            return Err(MailError::ParseError(
                "mime message must contain subject, body, inline resource, or attachment"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn resolve_from_mailbox(&self, from: Option<&str>) -> Result<Mailbox, MailError> {
        match from.or_else(|| self.mail_properties.username()) {
            Some(from) => self.parse_mailbox(from),
            None => Err(MailError::ParseError(
                "mail sender address is required".to_string(),
            )),
        }
    }

    fn parse_mailbox(&self, mailbox: &str) -> Result<Mailbox, MailError> {
        mailbox
            .parse::<Mailbox>()
            .map_err(|error| MailError::ParseError(error.to_string()))
    }

    fn parse_content_type(&self, value: &str, fallback: &str) -> Result<ContentType, MailError> {
        ContentType::parse(value).or_else(|_| {
            ContentType::parse(fallback).map_err(|error| MailError::ParseError(error.to_string()))
        })
    }

    fn text_content_type(&self, html: bool) -> ContentType {
        let base = if html { "text/html" } else { "text/plain" };
        let with_charset = format!(
            "{base}; charset={}",
            self.mail_properties.default_encoding()
        );

        ContentType::parse(&with_charset).unwrap_or(if html {
            ContentType::TEXT_HTML
        } else {
            ContentType::TEXT_PLAIN
        })
    }

    fn lookup_mime_type<'a>(&'a self, filename: &str, default: &'a str) -> &'a str {
        let extension = filename
            .rsplit('.')
            .next()
            .filter(|value| *value != filename)
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let Some(extension) = extension else {
            return default;
        };

        let from_properties = self
            .properties
            .as_ref()
            .and_then(|properties| properties.get(&format!("mime.{extension}")))
            .map(String::as_str);

        from_properties.unwrap_or(default)
    }

    fn chrono_to_system_time(
        sent_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<SystemTime, MailError> {
        let seconds = sent_date.timestamp();
        let nanos = sent_date.timestamp_subsec_nanos();

        if seconds >= 0 {
            Ok(UNIX_EPOCH
                + Duration::from_secs(seconds as u64)
                + Duration::from_nanos(u64::from(nanos)))
        } else {
            UNIX_EPOCH
                .checked_sub(Duration::from_secs(seconds.unsigned_abs()))
                .and_then(|time| time.checked_add(Duration::from_nanos(u64::from(nanos))))
                .ok_or_else(|| MailError::ParseError("mail sent date is out of range".to_string()))
        }
    }

    fn is_reserved_header(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "from"
                | "reply-to"
                | "to"
                | "cc"
                | "bcc"
                | "subject"
                | "date"
                | "mime-version"
                | "content-type"
                | "content-transfer-encoding"
        )
    }

    fn map_transport_error(error: lettre::transport::smtp::Error) -> MailError {
        let status = error.status().map(u16::from);

        if matches!(status, Some(530 | 534 | 535)) {
            return MailError::AuthenticationError(error.to_string());
        }

        MailError::SendError(error.to_string())
    }
}

#[async_trait]
impl MailService for DefaultMailService {
    fn create_mime_message(&self) -> MimeMessage {
        let _ = &self.default_file_type_map;
        MimeMessage::new()
    }

    async fn send_mime_message(&self, mime_message: MimeMessage) -> Result<(), MailError> {
        self.send_mime_messages(vec![mime_message]).await
    }

    async fn send_mime_messages(&self, mime_messages: Vec<MimeMessage>) -> Result<(), MailError> {
        let messages = mime_messages
            .into_iter()
            .map(|message| self.mime_message_to_lettre(message))
            .collect::<Result<Vec<_>, _>>()?;

        self.do_send(messages).await
    }
}

#[async_trait]
impl MailSender for DefaultMailService {
    async fn send(&self, message: DefaultMailMessage) -> Result<(), MailError> {
        self.send_batch(vec![message]).await
    }

    async fn send_batch(&self, messages: Vec<DefaultMailMessage>) -> Result<(), MailError> {
        let messages = messages
            .into_iter()
            .map(|message| self.simple_message_to_lettre(message))
            .collect::<Result<Vec<_>, _>>()?;

        self.do_send(messages).await
    }
}

impl Deref for DefaultMailService {
    type Target = AsyncSmtpTransport<Tokio1Executor>;

    fn deref(&self) -> &Self::Target {
        &self.smtp_transport
    }
}

impl_service!(DefaultMailService);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mime_message_helper::MimeMessageHelper;

    fn create_service() -> DefaultMailService {
        let mut properties = MailProperties::default();
        properties.set_host(Some("localhost".to_string()));
        properties.set_username(Some("sender@example.com".to_string()));

        DefaultMailService::new(properties).expect("mail service should be created")
    }

    #[tokio::test]
    async fn builds_html_message_with_attachment_and_inline_resource() {
        let service = create_service();
        let mut helper =
            MimeMessageHelper::from_message(service.create_mime_message(), true, None::<String>);

        helper
            .set_to(["receiver@example.com"])
            .expect("recipient should be valid");
        helper
            .set_subject("Multipart mail")
            .expect("subject should be valid");
        helper
            .set_texts("plain body", "<p>html body <img src=\"cid:logo\"></p>")
            .expect("plain and html alternatives should be accepted");
        helper
            .add_inline("logo", vec![1, 2, 3], "image/png")
            .expect("inline resource should be accepted");
        helper
            .add_attachment("report.txt", b"attachment-body".as_slice(), "text/plain")
            .expect("attachment should be accepted");

        let message = service
            .mime_message_to_lettre(helper.into_message())
            .expect("mime message should be converted to lettre");

        let formatted = String::from_utf8(message.formatted())
            .expect("formatted lettre message should be utf8");

        assert!(formatted.contains("multipart/mixed"));
        assert!(formatted.contains("multipart/alternative"));
        assert!(formatted.contains("multipart/related"));
        assert!(formatted.contains("text/html"));
        assert!(formatted.contains("Content-Disposition: attachment"));
        assert!(formatted.contains("report.txt"));
        assert!(formatted.contains("Content-ID: <logo>"));
    }
}
