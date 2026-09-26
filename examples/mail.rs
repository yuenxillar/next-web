use std::sync::Arc;

use axum::{Json, extract::Multipart, http::StatusCode};
use next_web::{
    Application, NextWebApplication, extract::find_singleton::FindSingleton,
    macros::bind::post_mapping,
};
use next_web_mail::{mail_service::MailService, mime_message_helper::MimeMessageHelper};

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

#[derive(Debug, serde::Deserialize)]
struct HtmlMailRequest {
    from: Option<String>,
    to: Vec<String>,
    subject: String,
    text: Option<String>,
    html: String,
}

#[derive(Debug)]
struct UploadedAttachment {
    filename: String,
    content_type: String,
    bytes: Vec<u8>,
}

#[post_mapping(path = "/mail/html")]
async fn send_html_mail(
    FindSingleton(mail_service): FindSingleton<Arc<dyn MailService>>,
    Json(request): Json<HtmlMailRequest>,
) -> Result<&'static str, (StatusCode, String)> {
    send_html_mail_inner(mail_service, request).await
}

/// Sends the mail of the request, which is the body of the handler above.
///
/// The body of a handler of a mapping macro is added to the function that
/// serves the request, so the errors of the body are reported by a function of
/// its own.
///
/// # Arguments
///
/// * `mail_service` - The service the mail is sent with.
/// * `request` - The request of the mail.
async fn send_html_mail_inner(
    mail_service: Arc<dyn MailService>,
    request: HtmlMailRequest,
) -> Result<&'static str, (StatusCode, String)> {
    let mut helper =
        MimeMessageHelper::from_message(mail_service.create_mime_message(), true, None);

    apply_mail_headers(
        &mut helper,
        request.from,
        request.to,
        request.subject,
        request.text,
        Some(request.html),
    )?;

    mail_service
        .send_mime_message(helper.into_message())
        .await
        .map_err(internal_error)?;

    Ok("mail send ok!")
}

#[post_mapping(path = "/mail/upload")]
async fn send_mail_with_upload(
    FindSingleton(mail_service): FindSingleton<Arc<dyn MailService>>,
    multipart: Multipart,
) -> Result<&'static str, (StatusCode, String)> {
    send_mail_with_upload_inner(mail_service, multipart).await
}

/// Sends the mail of the uploaded files, which is the body of the handler
/// above.
///
/// # Arguments
///
/// * `mail_service` - The service the mail is sent with.
/// * `multipart` - The uploaded files of the mail.
async fn send_mail_with_upload_inner(
    mail_service: Arc<dyn MailService>,
    mut multipart: Multipart,
) -> Result<&'static str, (StatusCode, String)> {
    let mut from = None;
    let mut to = Vec::new();
    let mut subject = None;
    let mut text = None;
    let mut html = None;
    let mut attachments = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(internal_error)? {
        let name = field.name().unwrap_or_default().to_string();

        match name.as_str() {
            "file" => {
                let filename = field.file_name().unwrap_or("upload.bin").to_string();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let bytes = field.bytes().await.map_err(internal_error)?.to_vec();

                attachments.push(UploadedAttachment {
                    filename,
                    content_type,
                    bytes,
                });
            }
            "from" => {
                from = Some(field.text().await.map_err(internal_error)?);
            }
            "to" => {
                let value = field.text().await.map_err(internal_error)?;
                to.extend(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                );
            }
            "subject" => {
                subject = Some(field.text().await.map_err(internal_error)?);
            }
            "text" => {
                text = Some(field.text().await.map_err(internal_error)?);
            }
            "html" => {
                html = Some(field.text().await.map_err(internal_error)?);
            }
            _ => {
                let _ = field.bytes().await.map_err(internal_error)?;
            }
        }
    }

    let subject = subject.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "multipart field `subject` is required".to_string(),
        )
    })?;

    if text.is_none() && html.is_none() {
        Err((
            StatusCode::BAD_REQUEST,
            "multipart field `text` or `html` is required".to_string(),
        ))?;
    }

    let mut helper =
        MimeMessageHelper::from_message(mail_service.create_mime_message(), true, None::<String>);

    apply_mail_headers(&mut helper, from, to, subject, text, html)?;

    for attachment in attachments {
        helper
            .add_attachment(
                &attachment.filename,
                attachment.bytes,
                &attachment.content_type,
            )
            .map_err(bad_request)?;
    }

    mail_service
        .send_mime_message(helper.into_message())
        .await
        .map_err(internal_error)?;

    Ok("mail sent")
}

fn apply_mail_headers(
    helper: &mut MimeMessageHelper,
    from: Option<String>,
    to: Vec<String>,
    subject: String,
    text: Option<String>,
    html: Option<String>,
) -> Result<(), (StatusCode, String)> {
    if let Some(from) = from {
        helper.set_from(from).map_err(bad_request)?;
    }

    if to.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "at least one `to` recipient is required".to_string(),
        ));
    }

    helper.set_to(to).map_err(bad_request)?;
    helper.set_subject(subject).map_err(bad_request)?;

    match (text, html) {
        (Some(text), Some(html)) => helper.set_texts(text, html).map_err(bad_request)?,
        (Some(text), None) => helper.set_text(text, false).map_err(bad_request)?,
        (None, Some(html)) => helper.set_text(html, true).map_err(bad_request)?,
        (None, None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "`text` or `html` content is required".to_string(),
            ));
        }
    }

    Ok(())
}

fn bad_request(error: impl ToString) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, error.to_string())
}

fn internal_error(error: impl ToString) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
