//! A transport-independent snapshot of an HTTP response.
//!
//! [`HttpResponseShare`] mirrors the parts of a response that filters,
//! background tasks, and application events need after the request scope has
//! ended: the HTTP version, the status code, the headers, the captured body
//! bytes, and the committed flag. It implements [`HttpResponse`] as well, so a
//! snapshot can be treated like any other response.

use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version, header::LOCATION};

use crate::{http::Cookie, http::HttpResponse};

/// A cloneable, transport-independent snapshot of an HTTP response.
///
/// A snapshot is created from a live response with [`HttpResponse::shared`] or
/// [`HttpResponseShare::from`]. It owns its header map and body bytes, so it can
/// outlive the response it was taken from and be passed across threads.
///
/// # Examples
///
/// ```
/// use axum::{body::Body, http::StatusCode, response::Response};
/// use next_web_core::http::HttpResponseShare;
/// use next_web_core::http::HttpResponse;
///
/// let response = Response::builder()
///     .status(StatusCode::CREATED)
///     .header("x-trace-id", "abc")
///     .body(Body::empty())
///     .unwrap();
///
/// let snapshot = HttpResponseShare::from(&response as &dyn HttpResponse);
/// assert_eq!(snapshot.status_code(), StatusCode::CREATED);
/// assert_eq!(snapshot.header("x-trace-id"), Some("abc"));
/// ```
#[derive(Clone, Debug)]
pub struct HttpResponseShare {
    /// The HTTP version of the response.
    version: Version,
    /// The status code of the response.
    status_code: StatusCode,
    /// Every header of the response, preserved in order.
    headers: HeaderMap<HeaderValue>,
    /// The body bytes captured so far.
    body: Vec<u8>,
    /// Whether the response has been committed.
    committed: bool,
}

impl HttpResponseShare {
    /// Creates an empty `200 OK` snapshot.
    ///
    /// The snapshot has no headers or body and is not committed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: Version::default(),
            status_code: StatusCode::OK,
            headers: HeaderMap::new(),
            body: Vec::new(),
            committed: false,
        }
    }

    /// Creates an empty snapshot with the given status code.
    #[must_use]
    pub fn with_status(status_code: StatusCode) -> Self {
        Self {
            status_code,
            ..Self::new()
        }
    }

    /// Returns the HTTP version of the snapshot.
    #[must_use]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns the status code of the snapshot.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    /// Returns the full header map of the snapshot.
    ///
    /// This exposes every header at once, which the per-name
    /// [`HttpResponse::header`] and [`HttpResponse::headers`] methods cannot do.
    #[must_use]
    pub fn header_map(&self) -> &HeaderMap<HeaderValue> {
        &self.headers
    }

    /// Returns the captured body bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Returns whether the snapshot is committed.
    #[must_use]
    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

impl Default for HttpResponseShare {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpResponse for HttpResponseShare {
    fn version(&self) -> Version {
        self.version
    }

    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn set_status_code(&mut self, status_code: StatusCode) {
        self.status_code = status_code;
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }

    fn header_map(&self) -> &HeaderMap<HeaderValue> {
        &self.headers
    }

    fn headers(&self, name: &str) -> Option<Vec<&str>> {
        let values: Vec<&str> = self
            .headers
            .get_all(name)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect();
        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    }

    /// Appends `value` to the values of `name`.
    ///
    /// Returns `false` when `name` is not a valid header name or `value` cannot
    /// be encoded as a header value; nothing is changed in that case.
    fn append_header(&mut self, name: &str, value: &str) -> bool {
        let Ok(value) = HeaderValue::from_str(value) else {
            return false;
        };

        match HeaderName::from_bytes(name.as_bytes()) {
            Ok(name) => {
                self.headers.append(name, value);
                true
            }
            Err(_) => false,
        }
    }

    fn contains_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }

    /// Inserts `value`, replacing every existing value of `name`.
    ///
    /// Returns the previous value as a `String` when one was present, or an
    /// empty string when the header was absent. Returns `None` only when `name`
    /// or `value` is invalid, in which case nothing is changed.
    fn insert_header(&mut self, name: &str, value: &str) -> Option<String> {
        let Ok(name) = HeaderName::from_bytes(name.as_bytes()) else {
            return None;
        };

        value.parse().ok().map(|value| {
            self.headers
                .insert(name, value)
                .and_then(|previous| previous.to_str().ok().map(ToString::to_string))
                .unwrap_or_default()
        })
    }

    fn remove_header(&mut self, name: &str) -> Option<String> {
        HeaderName::from_bytes(name.as_bytes())
            .ok()
            .and_then(|name| self.headers.remove(name))
            .and_then(|value| value.to_str().ok().map(ToString::to_string))
    }

    fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
        // The trait promises that writing a body commits the response, and the
        // implementation on an axum `Response` does commit. A snapshot that did
        // not would report a different state for the same write.
        self.committed = true;
    }

    fn body(&self) -> &[u8] {
        &self.body
    }

    /// Turns the snapshot into a `303 See Other` redirect to `url`.
    ///
    /// An invalid `url` is ignored and the snapshot is left unchanged.
    fn set_redirect(&mut self, url: &str) {
        if let Ok(url) = HeaderValue::from_str(url) {
            self.status_code = StatusCode::SEE_OTHER;
            self.headers.insert(LOCATION, url);
        }
    }

    /// Appends `cookie` as a `Set-Cookie` header.
    fn add_cookie(&mut self, cookie: Cookie) {
        let _ = self.append_header("set-cookie", &cookie.to_string());
    }

    fn is_committed(&self) -> bool {
        self.committed
    }

    fn commit(&mut self) {
        self.committed = true;
    }

    fn shared(&mut self) -> &HttpResponseShare {
        self
    }
}

impl From<&dyn HttpResponse> for HttpResponseShare {
    /// Captures an owned snapshot of `value`.
    ///
    /// The version, status code, full header map, body bytes (when the source
    /// exposes them), and committed flag are copied; the result is independent
    /// of the source afterwards.
    fn from(value: &dyn HttpResponse) -> Self {
        Self {
            version: value.version(),
            status_code: value.status_code(),
            headers: value.header_map().clone(),
            body: value.body().to_vec(),
            committed: value.is_committed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{StatusCode, Version},
        response::Response,
    };

    use super::HttpResponseShare;
    use crate::{http::Cookie, http::HttpResponse};

    #[test]
    fn a_new_snapshot_defaults_to_ok() {
        let snapshot = HttpResponseShare::new();

        assert_eq!(snapshot.status_code(), StatusCode::OK);
        assert_eq!(snapshot.version(), Version::HTTP_11);
        assert!(!snapshot.is_committed());
        assert!(snapshot.header_map().is_empty());
        assert!(snapshot.body().is_empty());
    }

    #[test]
    fn headers_support_lookup_append_insert_and_remove() {
        let mut snapshot = HttpResponseShare::with_status(StatusCode::NO_CONTENT);

        assert!(snapshot.append_header("x-tag", "a"));
        assert!(snapshot.append_header("x-tag", "b"));
        assert_eq!(snapshot.headers("x-tag"), Some(vec!["a", "b"]));
        assert_eq!(snapshot.header("x-tag"), Some("a"));
        assert!(snapshot.contains_header("x-tag"));

        assert_eq!(snapshot.insert_header("x-tag", "c"), Some("a".to_string()));
        assert_eq!(snapshot.header("x-tag"), Some("c"));
        assert_eq!(snapshot.remove_header("x-tag"), Some("c".to_string()));
        assert!(!snapshot.contains_header("x-tag"));
    }

    #[test]
    fn invalid_names_and_values_are_rejected() {
        let mut snapshot = HttpResponseShare::new();

        assert!(!snapshot.append_header("bad name", "value"));
        assert!(!snapshot.append_header("x-tag", "bad\nvalue"));
        assert!(snapshot.insert_header("bad name", "value").is_none());
        assert!(snapshot.header_map().is_empty());
    }

    #[test]
    fn redirects_and_cookies_are_written_to_the_header_map() {
        let mut snapshot = HttpResponseShare::new();

        snapshot.set_redirect("/login");
        assert_eq!(snapshot.status_code(), StatusCode::SEE_OTHER);
        assert_eq!(snapshot.header("location"), Some("/login"));

        snapshot.add_cookie(Cookie::new("sid", Some("abc".to_string())));
        assert!(snapshot.header("set-cookie").is_some());
    }

    #[test]
    fn a_snapshot_captures_the_state_of_a_response() {
        let mut response = Response::builder()
            .status(StatusCode::CREATED)
            .version(Version::HTTP_2)
            .header("x-trace-id", "abc")
            .body(Body::empty())
            .unwrap();
        response.commit();

        let snapshot = HttpResponseShare::from(&response as &dyn HttpResponse);

        assert_eq!(snapshot.status_code(), StatusCode::CREATED);
        assert_eq!(snapshot.version(), Version::HTTP_2);
        assert_eq!(snapshot.header("x-trace-id"), Some("abc"));
        assert!(snapshot.is_committed());
    }

    #[test]
    fn set_body_is_visible_through_the_body_accessor() {
        let mut snapshot = HttpResponseShare::new();

        snapshot.set_body(b"hello".to_vec());

        assert_eq!(snapshot.body(), b"hello");
    }

    #[test]
    fn setting_a_body_commits_the_snapshot_too() {
        let mut snapshot = HttpResponseShare::new();
        assert!(!snapshot.is_committed());

        snapshot.set_body(b"hello".to_vec());

        assert!(
            snapshot.is_committed(),
            "the trait promises that writing a body commits the response, and an \
             axum response does commit — a snapshot must not report a different state \
             for the same write"
        );
    }
}
