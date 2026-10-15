use std::{borrow::Cow, collections::HashMap, net::SocketAddr, sync::Arc};

use axum::http::header::ACCEPT_LANGUAGE;
use axum::http::{HeaderMap, HeaderValue, Uri, header::CONTENT_TYPE, uri::Scheme};
use headers::{Cookie as HeaderCookie, HeaderMapExt, Host};
use next_web_context::Locale;

use crate::{
    any::AnyValue,
    http::{Cookie, HttpMethod, HttpVersion, session::HttpSession},
    http::{HttpRequest, ResourceDispatcher},
    web::ViewModel,
};

/// An owned, transport-independent snapshot of an HTTP request.
///
/// A snapshot is taken from a live request with [`HttpRequest::shared`] or
/// `HttpRequestShare::from`, and mirrors the parts a filter, a background task or
/// an application event needs after the request scope has ended: the request line,
/// the headers, the cookies, the buffered body, the attributes, the view model and
/// the peer address.
///
/// The session is **not** part of a snapshot, and neither is the resource
/// dispatcher: both outlive a single request and are reached through the context
/// instead. The context path a filter stored on the request is not carried
/// either. Those methods answer `None`, which is what the request would answer
/// when it had none.
pub struct HttpRequestShare {
    version: HttpVersion,
    method: HttpMethod,
    uri: Uri,
    body: Option<Vec<u8>>,
    headers: Arc<HeaderMap<HeaderValue>>,
    cookies: Vec<Cookie>,
    attributes: HashMap<String, AnyValue>,
    view_model: Option<ViewModel>,
    remote_addr: Option<SocketAddr>,
}

impl HttpRequestShare {}

impl HttpRequest for HttpRequestShare {
    fn session(&self) -> Option<&dyn HttpSession> {
        None
    }

    fn session_mut(&mut self, _create: bool) -> Option<&mut dyn HttpSession> {
        None
    }

    fn change_session_id(&mut self) -> String {
        String::new()
    }

    fn is_requested_session_id_valid(&self) -> bool {
        false
    }

    fn requested_session_id(&self) -> Option<&str> {
        None
    }

    fn cookie(&self) -> Option<&Cookie> {
        self.cookies()?.first()
    }

    fn cookies(&self) -> Option<&[Cookie]> {
        Some(&self.cookies)
    }

    fn attributes(&self) -> Option<&HashMap<String, AnyValue>> {
        Some(&self.attributes)
    }

    fn view_model(&self) -> Option<&ViewModel> {
        self.view_model.as_ref()
    }

    fn view_model_mut(&mut self) -> Option<&mut ViewModel> {
        self.view_model.as_mut()
    }

    fn set_view_model(&mut self, model: ViewModel) {
        self.view_model = Some(model);
    }

    fn remove_view_model(&mut self) {
        self.view_model = None;
    }

    fn resource_dispatcher(&self) -> Option<&dyn ResourceDispatcher> {
        None
    }

    fn method(&self) -> HttpMethod {
        self.method.clone()
    }

    fn version(&self) -> HttpVersion {
        self.version
    }

    fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.headers
    }

    fn header(&self, header_name: &str) -> Option<&str> {
        self.headers.get(header_name).and_then(|v| v.to_str().ok())
    }

    fn header_values(&self, header_name: &str) -> Vec<&str> {
        self.headers
            .get_all(header_name)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect()
    }

    fn header_names(&self) -> Vec<&str> {
        self.headers.keys().map(|k| k.as_str()).collect()
    }

    fn uri(&self) -> &Uri {
        &self.uri
    }

    fn query(&self) -> Option<&str> {
        self.uri.query()
    }

    fn parameter(&self, name: &str) -> Option<&str> {
        let query = self.query()?;
        query.split('&').find_map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (key == name).then_some(value)
        })
    }

    fn parameters(&self) -> Option<Vec<(Cow<'_, str>, Cow<'_, str>)>> {
        let query = self.query()?;
        Some(form_urlencoded::parse(query.as_bytes()).collect())
    }

    fn parameter_values(&self, name: &str) -> Option<Vec<Cow<'_, str>>> {
        self.parameters().map(|params| {
            params
                .into_iter()
                .filter(|(key, _)| *key == name)
                .map(|(_, value)| value)
                .collect()
        })
    }

    fn path(&self) -> &str {
        self.uri.path()
    }

    fn host(&self) -> Option<&str> {
        self.uri.host()
    }

    fn scheme(&self) -> Option<&str> {
        self.uri.scheme().map(|s| s.as_str())
    }

    fn server_port(&self) -> Option<u16> {
        self.uri.port_u16()
    }

    fn server_name(&self) -> Option<String> {
        self.headers
            .typed_get::<Host>()
            .map(|host| host.hostname().to_string())
    }

    /// Returns `None`.
    ///
    /// A context path a filter stored on the request is not part of what a
    /// snapshot carries, so a snapshot never reports one.
    fn context_path(&self) -> Option<&str> {
        None
    }

    fn locale(&self) -> Option<Locale> {
        self.header(ACCEPT_LANGUAGE.as_str())
            .and_then(Locale::from_accept_language)
    }

    fn locales(&self) -> Option<Vec<Locale>> {
        let accept_language = self.header(ACCEPT_LANGUAGE.as_str())?;
        let locales = Locale::parse_accept_language(accept_language);

        if locales.is_empty() {
            Some(vec![Locale::system_locale()])
        } else {
            Some(locales)
        }
    }

    fn get_attribute(&self, name: &str) -> Option<&AnyValue> {
        self.attributes.get(name)
    }

    fn remove_attribute(&mut self, name: &str) {
        self.attributes.remove(name);
    }

    fn set_attribute(&mut self, name: &str, value: AnyValue) {
        self.attributes.insert(name.to_string(), value);
    }

    fn ready(&mut self) {}

    fn clean_up(&mut self) {
        self.attributes.clear();
    }

    fn is_secure(&self) -> bool {
        self.uri.scheme() == Some(&Scheme::HTTPS)
    }

    fn remote_addr(&self) -> Option<&SocketAddr> {
        self.remote_addr.as_ref()
    }

    fn shared(&mut self) -> &HttpRequestShare {
        self
    }

    fn content_type(&self) -> Option<&str> {
        self.headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok())
    }

    /// Returns the body the request had when the snapshot was taken.
    ///
    /// The body is only there when the request exposed it — a layer buffers the
    /// body for the requests whose filters read it — which is the same condition
    /// the live request reports, so a snapshot answers exactly as the request it
    /// came from did.
    fn body_bytes(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }
}

impl From<&dyn HttpRequest> for HttpRequestShare {
    fn from(req: &dyn HttpRequest) -> Self {
        Self {
            version: req.version(),
            method: req.method(),
            uri: req.uri().clone(),
            body: req.body_bytes().map(<[u8]>::to_vec),
            headers: Arc::new(req.headers().to_owned()),
            cookies: parse_cookies(req.headers()),
            attributes: req.attributes().cloned().unwrap_or_default(),
            view_model: req.view_model().cloned(),
            remote_addr: req.remote_addr().copied(),
        }
    }
}

impl Clone for HttpRequestShare {
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            method: self.method.to_owned(),
            uri: self.uri.to_owned(),
            body: self.body.clone(),
            headers: Arc::clone(&self.headers),
            cookies: self.cookies.clone(),
            attributes: self.attributes.clone(),
            view_model: self.view_model.clone(),
            remote_addr: self.remote_addr,
        }
    }
}

fn parse_cookies(headers: &HeaderMap<HeaderValue>) -> Vec<Cookie> {
    let Some(cookie_header) = headers.typed_get::<HeaderCookie>() else {
        return Vec::new();
    };
    cookie_header
        .iter()
        .map(|(name, value)| Cookie::new(name, Some(value.to_string())))
        .collect()
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};

    use super::HttpRequestShare;
    use crate::{http::HttpRequest, http::RequestBody};

    #[test]
    fn a_snapshot_carries_the_attributes_of_the_request() {
        let mut request = Request::builder().body(Body::empty()).unwrap();
        request.set_attribute("user", "alice".into());

        let snapshot = HttpRequestShare::from(&request as &dyn HttpRequest);

        assert_eq!(
            snapshot
                .get_attribute("user")
                .and_then(|value| value.as_str()),
            Some("alice"),
            "an attribute a filter set stays readable once the request is snapshotted"
        );
    }

    #[test]
    fn a_snapshot_carries_the_body_the_request_exposed() {
        let mut request = Request::builder().body(Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(RequestBody::new(b"hello".to_vec()));

        assert_eq!(
            request.body_bytes(),
            Some(&b"hello"[..]),
            "the request exposes the buffered body"
        );

        let snapshot = HttpRequestShare::from(&request as &dyn HttpRequest);

        assert_eq!(
            snapshot.body_bytes(),
            Some(&b"hello"[..]),
            "the body the layer buffered travels with the snapshot"
        );
    }

    #[test]
    fn a_snapshot_of_a_request_without_a_body_reports_none() {
        let request = Request::builder().body(Body::empty()).unwrap();

        let snapshot = HttpRequestShare::from(&request as &dyn HttpRequest);

        assert!(snapshot.body_bytes().is_none());
    }
}
