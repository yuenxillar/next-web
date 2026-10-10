//! The buffered body of a request.
//!
//! A transport hands the body of a request as a stream, which a filter cannot
//! read synchronously. The body is buffered by the web layer for the requests
//! whose filters have to read it, and it is then available to every filter
//! through
//! [`HttpRequest::body_bytes`](crate::traits::http::http_request::HttpRequest::body_bytes).

use std::{borrow::Cow, fmt};

use axum::{body::Bytes, extract::Request};

/// The content type of a url-encoded form.
pub const FORM_URLENCODED: &str = "application/x-www-form-urlencoded";

/// The content type of a JSON document.
pub const APPLICATION_JSON: &str = "application/json";

/// The largest body that is buffered by default, in bytes.
pub const DEFAULT_MAX_BUFFERED_BODY_BYTES: usize = 64 * 1024;

/// The bodies that are buffered for the filters of an application.
///
/// A filter reads the body of a request synchronously, so the body has to be
/// read before the filters run. Only the content types a filter reads are
/// buffered, and only up to a limit, so a request that streams a payload is left
/// untouched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyBufferPolicy {
    content_types: Vec<String>,
    max_bytes: usize,
}

impl BodyBufferPolicy {
    /// Creates the default policy: a url-encoded form and a JSON document, up
    /// to [`DEFAULT_MAX_BUFFERED_BODY_BYTES`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the largest body that is buffered.
    ///
    /// # Arguments
    ///
    /// * `max_bytes` - The limit, in bytes.
    pub fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    /// Adds a content type to the body the policy buffers.
    ///
    /// The type matches a content type that carries a parameter, such as
    /// `application/json;charset=UTF-8`.
    ///
    /// # Arguments
    ///
    /// * `content_type` - The content type to buffer.
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        let content_type = content_type.into();
        if !self.content_types.contains(&content_type) {
            self.content_types.push(content_type);
        }
        self
    }

    /// Removes a content type from the body the policy buffers.
    ///
    /// # Arguments
    ///
    /// * `content_type` - The content type to leave untouched.
    pub fn without_content_type(mut self, content_type: &str) -> Self {
        self.content_types
            .retain(|buffered| buffered != content_type);
        self
    }

    /// Returns the largest body that is buffered, in bytes.
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// Returns the content types the policy buffers.
    pub fn content_types(&self) -> &[String] {
        &self.content_types
    }

    /// Returns whether the policy buffers a body of the given content type.
    ///
    /// # Arguments
    ///
    /// * `content_type` - The content type of the request.
    pub fn accepts(&self, content_type: &str) -> bool {
        self.content_types
            .iter()
            .any(|buffered| content_type.starts_with(buffered.as_str()))
    }
}

impl Default for BodyBufferPolicy {
    fn default() -> Self {
        Self {
            content_types: vec![FORM_URLENCODED.to_string(), APPLICATION_JSON.to_string()],
            max_bytes: DEFAULT_MAX_BUFFERED_BODY_BYTES,
        }
    }
}

/// The error of a body that could not be buffered.
///
/// The body of a request that carries more than the limit of the policy is not
/// buffered: a filter cannot read a body that large, and reading it into memory
/// is what the limit prevents. The request is refused instead of being passed on
/// without its body, so an application never reads a truncated payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyBufferError {
    limit: usize,
    message: String,
}

impl BodyBufferError {
    /// Returns the limit the policy set, in bytes.
    pub fn limit(&self) -> usize {
        self.limit
    }
}

impl fmt::Display for BodyBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the body of the request is larger than the {} bytes that are buffered for the \
             filters: {}",
            self.limit, self.message
        )
    }
}

impl std::error::Error for BodyBufferError {}

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

/// Buffers the body of a request for the filters of an application.
///
/// The body of a request whose content type the policy does not name is left
/// untouched, and so is a request without a body, so a request that streams a
/// payload is not read into memory.
///
/// # Arguments
///
/// * `request` - The request of the layer.
/// * `policy` - The bodies the policy buffers.
///
/// # Errors
///
/// Returns a [`BodyBufferError`] when the body is larger than the limit of the
/// policy.
pub async fn buffer_request_body_for(
    request: Request,
    policy: &BodyBufferPolicy,
) -> Result<Request, BodyBufferError> {
    let content_type = request
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|content_type| content_type.to_str().ok());
    let Some(content_type) = content_type else {
        return Ok(request);
    };
    if !policy.accepts(content_type) {
        return Ok(request);
    }

    let (parts, body) = request.into_parts();
    match axum::body::to_bytes(body, policy.max_bytes).await {
        Ok(bytes) => {
            let mut request = Request::from_parts(parts, axum::body::Body::from(bytes.clone()));
            buffer_request_body(&mut request, bytes);

            Ok(request)
        }
        Err(error) => Err(BodyBufferError {
            limit: policy.max_bytes,
            message: error.to_string(),
        }),
    }
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

    use super::{
        buffer_request_body, buffer_request_body_for, form_parameters, BodyBufferPolicy,
        RequestBody, APPLICATION_JSON, FORM_URLENCODED,
    };
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

    /// Returns a request with the given content type and body.
    ///
    /// # Arguments
    ///
    /// * `content_type` - The content type of the request.
    /// * `body` - The body of the request.
    fn request(content_type: &str, body: &str) -> Request {
        Request::builder()
            .header("Content-Type", content_type)
            .body(Body::from(body.to_string()))
            .expect("the request is built")
    }

    #[test]
    fn the_default_policy_buffers_a_form_and_a_json_document() {
        let policy = BodyBufferPolicy::default();

        assert!(policy.accepts(FORM_URLENCODED));
        assert!(policy.accepts("application/x-www-form-urlencoded; charset=UTF-8"));
        assert!(policy.accepts(APPLICATION_JSON));
        assert!(policy.accepts("application/json;charset=UTF-8"));
        assert!(!policy.accepts("text/plain"));
        assert!(!policy.accepts("multipart/form-data; boundary=x"));
    }

    #[tokio::test]
    async fn a_form_body_is_buffered_for_the_filters() {
        let request = request(FORM_URLENCODED, "client_id=demo");

        let request = buffer_request_body_for(request, &BodyBufferPolicy::default())
            .await
            .expect("the body is buffered");

        assert_eq!(request.body_bytes(), Some("client_id=demo".as_bytes()));
    }

    #[tokio::test]
    async fn the_body_of_another_content_type_is_left_untouched() {
        let request = request("text/plain", "hello");

        let request = buffer_request_body_for(request, &BodyBufferPolicy::default())
            .await
            .expect("the request is passed on");

        assert_eq!(request.body_bytes(), None, "the body is not buffered");
    }

    #[tokio::test]
    async fn a_body_larger_than_the_limit_is_refused() {
        let policy = BodyBufferPolicy::default().with_max_bytes(4);
        let request = request(FORM_URLENCODED, "client_id=demo");

        let error = buffer_request_body_for(request, &policy)
            .await
            .expect_err("the body is larger than the limit");

        assert_eq!(error.limit(), 4);
        assert!(error.to_string().contains("larger than the 4 bytes"));
    }

    #[tokio::test]
    async fn a_request_without_a_content_type_is_left_untouched() {
        let request = Request::builder()
            .body(Body::from("client_id=demo"))
            .expect("the request is built");

        let request = buffer_request_body_for(request, &BodyBufferPolicy::default())
            .await
            .expect("the request is passed on");

        assert_eq!(request.body_bytes(), None);
    }
}
