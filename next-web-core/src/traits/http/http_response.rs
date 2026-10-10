//! The [`HttpResponse`] abstraction and its implementation for
//! [`axum::response::Response`].
//!
//! `HttpResponse` is the framework's servlet-style view of the response a
//! filter or handler is building. It exposes the status line, the headers, the
//! body, cookies, redirects, and the "committed" flag, while hiding the concrete
//! transport.
//!
//! # Committing a response
//!
//! Writing a body ([`HttpResponse::set_body`]) commits the response, which
//! records that the response is no longer safe to modify structurally. A filter
//! can test [`HttpResponse::is_committed`] and, for example, skip writing an
//! error page over a response that has already produced output.

use axum::http::header;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version},
    response::Response,
};

use crate::http::{Cookie, HttpResponseShare};

/// Extension marker recording that a response has been committed.
///
/// It is stored in the response extensions by [`HttpResponse::commit`] and read
/// back by [`HttpResponse::is_committed`].
#[derive(Clone, Copy)]
struct Committed;

/// The servlet-style view of an HTTP response.
///
/// The trait is implemented for [`axum::response::Response`], so the transport
/// types of the web layer can be used directly by filters and handlers, and for
/// [`HttpResponseShare`], which is an owned snapshot of a response.
///
/// Header names are matched case-insensitively, and methods that take a header
/// name validate it before touching the header map; an invalid name or value
/// leaves the response unchanged.
pub trait HttpResponse
where
    Self: Send,
{
    /// Returns the HTTP version of the response.
    fn version(&self) -> Version;

    /// Returns the status code of the response.
    fn status_code(&self) -> StatusCode;

    /// Replaces the status code of the response.
    fn set_status_code(&mut self, status_code: StatusCode);

    /// Returns the first value of the named header, when it is valid UTF-8.
    fn header(&self, name: &str) -> Option<&str>;

    /// Returns the full header map of the response.
    ///
    /// Unlike [`HttpResponse::header`] and [`HttpResponse::headers`], this
    /// exposes every header and lets callers copy the map as a whole (for
    /// example when building an [`HttpResponseShare`] snapshot).
    fn header_map(&self) -> &HeaderMap<HeaderValue>;

    /// Returns every value of the named header, skipping invalid UTF-8 values.
    ///
    /// Returns `None` when the header is absent (or has no valid UTF-8 value).
    fn headers(&self, name: &str) -> Option<Vec<&str>>;

    /// Appends `value` to the values of the named header.
    ///
    /// Returns `false` when the name is not a valid header name or the value
    /// cannot be encoded as a header value; nothing is changed in that case.
    fn append_header(&mut self, name: &str, value: &str) -> bool;

    /// Returns `true` when the response has at least one value for the named
    /// header.
    fn contains_header(&self, name: &str) -> bool;

    /// Inserts `value`, replacing every existing value of the named header.
    ///
    /// Returns the previous value as a `String` when one was present, or an
    /// empty string when the header was absent. Returns `None` only when the
    /// name or value is invalid, in which case nothing is changed.
    fn insert_header(&mut self, name: &str, value: &str) -> Option<String>;

    /// Removes the named header and returns its previous value, if any.
    fn remove_header(&mut self, name: &str) -> Option<String>;

    /// Replaces the body of the response and commits it.
    ///
    /// Implementations may also mark the response committed, as writing the
    /// body is the point after which the response is sent.
    fn set_body(&mut self, body: Vec<u8>);

    /// Returns the body bytes of the response, when they can be borrowed.
    ///
    /// The default returns an empty slice because a streaming body (such as the
    /// body of an [`axum::response::Response`]) cannot be borrowed without being
    /// consumed. Implementations that buffer their body, like
    /// [`HttpResponseShare`], override this method.
    fn body(&self) -> &[u8] {
        &[]
    }

    /// Turns the response into a redirect to `url`.
    ///
    /// An invalid `url` is ignored and the response is left unchanged.
    fn set_redirect(&mut self, url: &str);

    /// Appends `cookie` to the `Set-Cookie` headers of the response.
    fn add_cookie(&mut self, cookie: Cookie);

    /// Returns `true` once the response has been committed.
    fn is_committed(&self) -> bool;

    /// Marks the response as committed.
    fn commit(&mut self);

    /// Returns a transport-independent snapshot of the response.
    ///
    /// The snapshot is produced with [`HttpResponseShare::from`] and stored in
    /// the response extensions, so it can be cloned and outlive the request.
    fn shared(&mut self) -> &HttpResponseShare;
}

/// Implements the servlet-style view on axum's [`Response`].
///
/// Headers and the status line are delegated to the underlying
/// [`http::Response`], and the committed flag is tracked in the response
/// extensions.
impl HttpResponse for Response {
    fn version(&self) -> Version {
        self.version()
    }

    fn status_code(&self) -> StatusCode {
        self.status()
    }

    fn set_status_code(&mut self, status_code: StatusCode) {
        *self.status_mut() = status_code;
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
    }

    fn header_map(&self) -> &HeaderMap<HeaderValue> {
        self.headers()
    }

    fn headers(&self, name: &str) -> Option<Vec<&str>> {
        let values: Vec<&str> = self
            .headers()
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

    fn append_header(&mut self, name: &str, value: &str) -> bool {
        let Ok(value) = HeaderValue::from_str(value) else {
            return false;
        };

        match HeaderName::from_bytes(name.as_bytes()) {
            Ok(name) => {
                self.headers_mut().append(name, value);
                true
            }
            Err(_) => false,
        }
    }

    fn contains_header(&self, name: &str) -> bool {
        self.headers().contains_key(name)
    }

    fn insert_header(&mut self, name: &str, value: &str) -> Option<String> {
        let name = match HeaderName::from_bytes(name.as_bytes()) {
            Ok(name) => name,
            Err(_) => return None,
        };

        value.parse().ok().map(|value| {
            self.headers_mut()
                .insert(name, value)
                .and_then(|previous| previous.to_str().ok().map(ToString::to_string))
                .unwrap_or_default()
        })
    }

    fn remove_header(&mut self, name: &str) -> Option<String> {
        HeaderName::from_bytes(name.as_bytes())
            .ok()
            .and_then(|name| self.headers_mut().remove(name))
            .and_then(|value| value.to_str().ok().map(ToString::to_string))
    }

    fn set_body(&mut self, body: Vec<u8>) {
        *self.body_mut() = Body::from(body);
        self.commit();
    }

    fn set_redirect(&mut self, url: &str) {
        if let Ok(url) = HeaderValue::from_str(url) {
            *self.status_mut() = StatusCode::SEE_OTHER;
            self.headers_mut().insert(header::LOCATION, url);
        }
    }

    fn add_cookie(&mut self, cookie: Cookie) {
        let _ = self.append_header("set-cookie", &cookie.to_string());
    }

    fn is_committed(&self) -> bool {
        self.extensions().get::<Committed>().is_some()
    }

    fn commit(&mut self) {
        self.extensions_mut().insert(Committed);
    }

    fn shared(&mut self) -> &HttpResponseShare {
        // Refresh the snapshot on every call so it always reflects the current
        // response. The returned reference borrows `self`, so callers cannot
        // hold it across another `shared` call, which keeps the replacement
        // sound and the snapshot free of stale headers.
        let snapshot = HttpResponseShare::from(self as &dyn HttpResponse);
        self.extensions_mut().insert(snapshot);

        self.extensions()
            .get::<HttpResponseShare>()
            .expect("HttpResponseShare was just inserted")
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{StatusCode, Version},
        response::Response,
    };

    use super::{HttpResponse, HttpResponseShare};
    use crate::http::Cookie;

    fn empty_response() -> Response {
        Response::builder().body(Body::empty()).unwrap()
    }

    #[test]
    fn the_status_line_is_readable_and_settable() {
        let mut response = empty_response();

        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.version(), Version::HTTP_11);

        response.set_status_code(StatusCode::CREATED);
        assert_eq!(response.status_code(), StatusCode::CREATED);
    }

    #[test]
    fn headers_support_lookup_append_insert_and_remove() {
        let mut response = empty_response();

        assert!(response.append_header("x-tag", "a"));
        assert!(response.append_header("x-tag", "b"));
        assert_eq!(
            HttpResponse::headers(&response, "x-tag"),
            Some(vec!["a", "b"])
        );
        assert_eq!(response.header("x-tag"), Some("a"));
        assert!(response.contains_header("x-tag"));

        assert_eq!(response.insert_header("x-tag", "c"), Some("a".to_string()));
        assert_eq!(response.header("x-tag"), Some("c"));
        assert_eq!(response.remove_header("x-tag"), Some("c".to_string()));
        assert!(!response.contains_header("x-tag"));
    }

    #[test]
    fn invalid_names_and_values_are_rejected() {
        let mut response = empty_response();

        assert!(!response.append_header("bad name", "value"));
        assert!(!response.append_header("x-tag", "bad\nvalue"));
        assert!(response.insert_header("bad name", "value").is_none());
        assert!(response.header_map().is_empty());
    }

    #[test]
    fn setting_a_body_commits_the_response() {
        let mut response = empty_response();

        assert!(!response.is_committed());
        response.set_body(b"hello".to_vec());
        assert!(response.is_committed());

        // A streaming body cannot be borrowed, so the default accessor is empty.
        assert!(HttpResponse::body(&response).is_empty());
    }

    #[test]
    fn redirects_and_cookies_are_written_to_the_response() {
        let mut response = empty_response();

        response.set_redirect("/login");
        assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
        assert_eq!(response.header("location"), Some("/login"));

        response.add_cookie(Cookie::new("sid", Some("abc".to_string())));
        assert!(response.header("set-cookie").is_some());
    }

    #[test]
    fn the_shared_snapshot_reflects_the_current_response() {
        let mut response = Response::builder()
            .status(StatusCode::CREATED)
            .header("x-trace-id", "abc")
            .body(Body::empty())
            .unwrap();

        {
            let snapshot: &HttpResponseShare = response.shared();
            assert_eq!(snapshot.status_code(), StatusCode::CREATED);
            assert_eq!(snapshot.header("x-trace-id"), Some("abc"));
        }

        // A header added later is visible to a later snapshot, because the
        // snapshot is refreshed on each call.
        response.insert_header("x-extra", "1");
        let snapshot = response.shared();
        assert_eq!(snapshot.header("x-extra"), Some("1"));
    }
}
