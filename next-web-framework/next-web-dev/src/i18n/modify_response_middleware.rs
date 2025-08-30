use axum::{
    body::{Body, Bytes}, extract::{Request, State}, middleware::Next, response::Response
};
use http_body_util::BodyExt;
use next_web_core::traits::locale_resolver::LocaleResolver;

use crate::{i18n::locale::accept_header_locale_resolver::AcceptHeaderLocaleResolver, service::message_source_service::MessageSourceService};

#[allow(unused)]
pub(crate) async fn modify_response(
    State(message_source_service): State<MessageSourceService>,
    req: Request,
    next: Next,
) -> Response {

    let resolver = AcceptHeaderLocaleResolver;
    let locale = resolver.resolve_locale(&req);

    let resp = next.run(req).await;

    let (parts, body) = resp.into_parts();

    let  bytes = match body.collect().await {
        Ok(coll) => coll.to_bytes(),
        Err(_) => return Response::from_parts(parts, Body::empty())
    };

    // message=hello
    let code = String::from_utf8_lossy(&bytes[8..]).to_string();
    
    let msg = message_source_service.message_or_default(code, locale);

    let body = Bytes::from(format!("message={}", msg));

    Response::from_parts(parts, body.into())
}