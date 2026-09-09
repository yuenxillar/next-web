use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::http::{HeaderMap, HeaderValue, Uri, header::CONTENT_TYPE, uri::Scheme};
use headers::{Cookie as HeaderCookie, HeaderMapExt, Host};

use crate::{
    anys::any_value::AnyValue,
    autoconfigure::context::server_properties::GLOBAL_SERVER_PROPERTIES,
    http::{Cookie, HttpMethod, HttpVersion, auth_type::AuthType},
    traits::http::{HttpSession, http_request::HttpRequest, request_dispatcher::RequestDispatcher},
    util::locale::Locale,
};

pub struct HttpRequestShare {
    version: HttpVersion,
    method: HttpMethod,
    uri: Uri,
    body: (),
    headers: Arc<HeaderMap<HeaderValue>>,
    cookies: Vec<Cookie>,
    attributes: HashMap<String, AnyValue>,
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

    fn auth_type(&self) -> AuthType {
        AuthType::from_request(self)
    }

    fn cookie(&self) -> Option<&Cookie> {
        self.cookies()?.first()
    }

    fn cookies(&self) -> Option<&[Cookie]> {
        Some(&self.cookies)
    }

    fn request_dispatcher(&self, _default_failure_url: &str) -> Option<&dyn RequestDispatcher> {
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

    fn parameters(&self) -> Option<Vec<(&str, &str)>> {
        let query = self.query()?;
        Some(
            query
                .split('&')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let (key, value) = part.split_once('=').unwrap_or((part, ""));
                    (key, value)
                })
                .collect(),
        )
    }

    fn parameter_values(&self, name: &str) -> Option<Vec<&str>> {
        let query = self.query()?;
        Some(
            query
                .split('&')
                .filter_map(|part| {
                    let (key, value) = part.split_once('=').unwrap_or((part, ""));
                    (key == name).then_some(value)
                })
                .collect(),
        )
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

    fn context_path(&self) -> Option<&str> {
        GLOBAL_SERVER_PROPERTIES
            .get()
            .and_then(|var| var.context_path())
    }

    fn locale(&self) -> Option<Locale> {
        self.header("Accept-Language")
            .and_then(Locale::from_accept_language)
    }

    fn locales(&self) -> Option<Vec<Locale>> {
        let accept_language = self.header("Accept-Language")?;
        let locales: Vec<Locale> = accept_language
            .split(',')
            .filter_map(Locale::from_accept_language)
            .collect();
        if locales.is_empty() {
            Some(vec![Locale::default()])
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
}

impl From<&dyn HttpRequest> for HttpRequestShare {
    fn from(req: &dyn HttpRequest) -> Self {
        Self {
            version: req.version(),
            method: req.method(),
            uri: req.uri().clone(),
            body: (),
            headers: Arc::new(req.headers().to_owned()),
            cookies: parse_cookies(req.headers()),
            attributes: HashMap::new(),
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
            body: self.body,
            headers: Arc::clone(&self.headers),
            cookies: self.cookies.clone(),
            attributes: self.attributes.clone(),
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
