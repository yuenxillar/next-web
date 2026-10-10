//! The buffered body of a request.
//!
//! A transport hands the body of a request as a stream, which a filter cannot
//! read synchronously. The body is buffered by the web layer for the requests
//! whose filters have to read it, and it is then available to every filter
//! through
//! [`HttpRequest::body_bytes`](crate::traits::http::http_request::HttpRequest::body_bytes).

use std::borrow::Cow;

use axum::{body::Bytes, extract::Request};

/// The body of a request, buffered so that the filters can read it.
#[derive(Clone, Debug, Default)]
pub struct RequestBody(Bytes);

impl RequestBody {
    /// Creates a buffered body.
    ///
    /// # Arguments
    ///
    /// * `body` - The bytes of the body.
    pub fn new(body: impl Into<Bytes>) -> Self {
        Self(body.into())
    }

    /// Returns the buffered bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns the buffered bytes.
    pub fn bytes(&self) -> &Bytes {
        &self.0
    }
}

/// Installs `body` as the buffered body of `request`.
///
/// # Arguments
///
/// * `request` - The request the body belongs to.
/// * `body` - The bytes of the body.
pub fn buffer_request_body(request: &mut Request, body: impl Into<Bytes>) {
    request.extensions_mut().insert(RequestBody::new(body));
}

/// Returns the parameters of a url-encoded form body.
///
/// # Arguments
///
/// * `body` - The bytes of the form body.
pub fn form_parameters(body: &[u8]) -> Vec<(Cow<'_, str>, Cow<'_, str>)> {
    form_urlencoded::parse(body).collect()
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};

    use super::{buffer_request_body, form_parameters, RequestBody};
    use crate::traits::http::http_request::HttpRequest;

    #[test]
    fn buffers_the_body_of_a_request() {
        let mut request = Request::builder().body(Body::empty()).unwrap();
        buffer_request_body(&mut request, "a=1&b=2");

        assert_eq!(request.body_bytes(), Some("a=1&b=2".as_bytes()));
        assert_eq!(
            request
                .extensions()
                .get::<RequestBody>()
                .map(|body| body.as_bytes()),
            Some("a=1&b=2".as_bytes())
        );
    }

    #[test]
    fn a_request_without_a_buffered_body_has_none() {
        let request = Request::builder().body(Body::empty()).unwrap();

        assert_eq!(request.body_bytes(), None);
    }

    #[test]
    fn parses_the_parameters_of_a_form_body() {
        let parameters = form_parameters(b"client_id=demo&scope=read+write&state=a%2Fb");

        assert_eq!(
            parameters,
            vec![
                ("client_id".into(), "demo".into()),
                ("scope".into(), "read write".into()),
                ("state".into(), "a/b".into()),
            ]
        );
    }
}
