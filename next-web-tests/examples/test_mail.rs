use std::sync::Arc;

use axum::{Json, extract::Multipart, http::StatusCode};
use next_web::{application::Application, extract::find_singleton::FindSingleton};
use next_web_core::{ApplicationContext, async_trait, context::properties::ApplicationProperties};
use next_web_mail::{mail_service::MailService, mime_message_helper::MimeMessageHelper};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) {
    }

    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/mail/html", axum::routing::post(send_html_mail))
            .route("/mail/upload", axum::routing::post(send_mail_with_upload))
    }
}

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

async fn send_html_mail(
    FindSingleton(mail_service): FindSingleton<Arc<dyn MailService>>,
    Json(request): Json<HtmlMailRequest>,
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

async fn send_mail_with_upload(
    FindSingleton(mail_service): FindSingleton<Arc<dyn MailService>>,
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
        return Err((
            StatusCode::BAD_REQUEST,
            "multipart field `text` or `html` is required".to_string(),
        ));
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
    TestApplication::run().await;
}
