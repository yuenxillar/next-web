//! The layer that prepares a request before the filters and the routes of an
//! application run.

use axum::{extract::Request, middleware::Next, response::Response};
use next_web_core::traits::http::http_request::HttpRequest;

/// Prepares the per-request state of the request.
///
/// The attributes, the cookies, and the session of the client are read from the
/// request, so the filters and the handlers of the application can read the
/// state a previous request of the client left behind. The layer is installed
/// outside every other layer, so the state is available to all of them.
///
/// The layer does not finish the request: the request is consumed by the
/// service it is handed to, so a caller that only prepares a request cannot read
/// it once the response was produced. The layer that runs the filters of the
/// application finishes the request, because it owns the request until the
/// response is produced.
///
/// # Arguments
///
/// * `request` - The request of the layer.
/// * `next` - The rest of the application.
pub(crate) async fn prepare_request_layer(mut request: Request, next: Next) -> Response {
    HttpRequest::ready(&mut request);

    next.run(request).await
}
