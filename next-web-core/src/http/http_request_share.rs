use std::{net::SocketAddr, sync::Arc};

use axum::http::{
    HeaderMap, HeaderValue, Uri,
    uri::{PathAndQuery, Scheme},
};
use dashmap::DashMap;

use crate::{
    anys::any_value::AnyValue,
    http::{Cookie, HttpMethod, HttpVersion, auth_type::AuthType},
    traits::http::{HttpSession, http_request::HttpRequest, request_dispatcher::RequestDispatcher},
    util::locale::Locale,
};

pub struct HttpRequestShare {
    version: HttpVersion,
    method: HttpMethod,
    scheme: Scheme,
    path_and_query: Option<PathAndQuery>,
    body: (),
    headers: Arc<HeaderMap<HeaderValue>>,

    attributes: Arc<DashMap<String, AnyValue>>,
}

impl HttpRequestShare {}

impl HttpRequest for HttpRequestShare {
    fn session(&self) -> Option<&dyn HttpSession> {
        todo!()
    }

    fn session_mut(&mut self, create: bool) -> Option<&mut dyn HttpSession> {
        todo!()
    }

    fn change_session_id(&mut self) -> String {
        todo!()
    }

    fn is_requested_session_id_valid(&self) -> bool {
        todo!()
    }

    fn auth_type(&self) -> AuthType {
        todo!()
    }

    fn cookie(&self) -> Option<&Cookie> {
        todo!()
    }

    fn cookies(&self) -> Option<&[Cookie]> {
        todo!()
    }

    fn request_dispatcher(&self, default_failure_url: &str) -> Option<&dyn RequestDispatcher> {
        todo!()
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
        todo!()
    }

    fn query(&self) -> Option<&str> {
        self.path_and_query.as_ref().and_then(|pq| pq.query())
    }

    fn parameter(&self, name: &str) -> Option<&str> {
        todo!()
    }

    fn parameters(&self) -> Option<Vec<(&str, &str)>> {
        todo!()
    }

    fn parameter_values(&self, name: &str) -> Option<Vec<&str>> {
        todo!()
    }

    fn path(&self) -> &str {
        self.path_and_query
            .as_ref()
            .map(|pq| pq.path())
            .unwrap_or_default()
    }

    fn host(&self) -> Option<&str> {
        todo!()
    }

    fn scheme(&self) -> Option<&str> {
        todo!()
    }

    fn server_port(&self) -> Option<u16> {
        todo!()
    }

    fn server_name(&self) -> Option<String> {
        todo!()
    }

    fn context_path(&self) -> Option<&str> {
        todo!()
    }

    fn locale(&self) -> Option<&Locale> {
        todo!()
    }

    fn locales(&self) -> Option<Vec<&Locale>> {
        todo!()
    }

    fn get_attribute(&self, name: &str) -> Option<&AnyValue> {
        todo!()
    }

    fn remove_attribute(&mut self, name: &str) {
        self.attributes.remove(name);
    }

    fn set_attribute(&mut self, name: &str, value: AnyValue) {
        self.attributes.insert(name.to_string(), value);
    }

    fn ready(&mut self) {
        todo!()
    }

    fn clean_up(&mut self) {
        todo!()
    }

    fn is_secure(&self) -> bool {
        self.scheme == Scheme::HTTPS
    }

    fn remote_addr(&self) -> Option<&SocketAddr> {
        todo!()
    }
}

impl From<&dyn HttpRequest> for HttpRequestShare {
    fn from(req: &dyn HttpRequest) -> Self {
        Self {
            version: req.version(),
            method: req.method(),
            scheme: req.uri().scheme().map(Clone::clone).unwrap_or(Scheme::HTTP),
            path_and_query: req.uri().path_and_query().cloned(),
            body: (),
            headers: Arc::new(req.headers().to_owned()),
            attributes: Arc::new(DashMap::new()),
        }
    }
}

impl Clone for HttpRequestShare {
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            method: self.method.to_owned(),
            scheme: self.scheme.to_owned(),
            path_and_query: self.path_and_query.clone(),
            body: self.body,
            headers: Arc::clone(&self.headers),
            attributes: self.attributes.clone(),
        }
    }
}
