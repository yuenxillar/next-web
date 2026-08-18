use std::{collections::HashMap, str::FromStr, sync::Arc};

use next_web_core::http::{HeaderMap, HeaderValue, Uri};
use next_web_core::{
    http::{Cookie, HttpMethod},
    traits::http::http_request::HttpRequest,
    util::locale::Locale,
};

use crate::web::savedrequest::SavedRequest;

/// Provides request parameters, headers and cookies from either an original request or a
/// saved request.
///
/// Note that not all request parameters in the original request are emulated by this
/// wrapper. Nevertheless, the important data from the original request is emulated and
/// this should prove adequate for most purposes (in particular standard HTTP GET and POST
/// operations).
///
/// Added into a request by `RequestCacheAwareFilter`.
pub struct SavedRequestAwareWrapper<'a> {
    request: &'a mut dyn HttpRequest,
    cookies: Vec<Cookie>,
    headers: HashMap<String, Vec<String>>,
    parameters: HashMap<String, Vec<String>>,
    locales: Vec<Locale>,
    method: HttpMethod,
    uri: Uri,
}

impl<'a> SavedRequestAwareWrapper<'a> {
    /// Creates a new instance that reads parameters, headers and cookies from the saved
    /// request and delegates the remaining state to the wrapped request.
    ///
    /// # Arguments
    ///
    /// * `saved` - the saved request to source parameters, headers and cookies from
    /// * `request` - the original request being wrapped
    pub fn new(saved: Arc<dyn SavedRequest>, request: &'a mut dyn HttpRequest) -> Self {
        let headers = saved
            .get_header_names()
            .into_iter()
            .map(|name| {
                let key = name.to_ascii_lowercase();
                let values = saved.get_header_values(&name);
                (key, values)
            })
            .collect();

        let uri = request.uri().clone();
        Self {
            request,
            cookies: saved.get_cookies(),
            headers,
            parameters: saved.get_parameter_map(),
            locales: saved.get_locales(),
            method: HttpMethod::from_str(&saved.get_method()).unwrap_or(HttpMethod::GET),
            uri,
        }
    }
}

impl HttpRequest for SavedRequestAwareWrapper<'_> {
    fn session(&self) -> Option<&dyn next_web_core::traits::http::HttpSession> {
        self.request.session()
    }

    fn session_mut(
        &mut self,
        create: bool,
    ) -> Option<&mut dyn next_web_core::traits::http::HttpSession> {
        self.request.session_mut(create)
    }

    fn change_session_id(&mut self) -> String {
        self.request.change_session_id()
    }

    fn is_requested_session_id_valid(&self) -> bool {
        self.request.is_requested_session_id_valid()
    }

    fn requested_session_id(&self) -> Option<&str> {
        todo!()
    }

    fn auth_type(&self) -> next_web_core::http::auth_type::AuthType {
        self.request.auth_type()
    }

    fn cookie(&self) -> Option<&Cookie> {
        self.cookies.first()
    }

    fn cookies(&self) -> Option<&[Cookie]> {
        Some(self.cookies.as_slice())
    }

    fn request_dispatcher(
        &self,
        default_failure_url: &str,
    ) -> Option<&dyn next_web_core::traits::http::request_dispatcher::RequestDispatcher> {
        self.request.request_dispatcher(default_failure_url)
    }

    fn method(&self) -> HttpMethod {
        self.method.clone()
    }

    fn version(&self) -> next_web_core::http::HttpVersion {
        self.request.version()
    }

    fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.request.headers()
    }

    fn header(&self, header_name: &str) -> Option<&str> {
        self.headers
            .get(&header_name.to_ascii_lowercase())
            .and_then(|values| values.first())
            .map(String::as_str)
    }

    fn header_values(&self, header_name: &str) -> Vec<&str> {
        self.headers
            .get(&header_name.to_ascii_lowercase())
            .map(|values| values.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    fn header_names(&self) -> Vec<&str> {
        self.headers.keys().map(String::as_str).collect()
    }

    fn uri(&self) -> &Uri {
        &self.uri
    }

    fn query(&self) -> Option<&str> {
        self.uri.query()
    }

    fn parameter(&self, name: &str) -> Option<&str> {
        self.parameters
            .get(name)
            .and_then(|values| values.first())
            .map(String::as_str)
    }

    fn parameters(&self) -> Option<Vec<(&str, &str)>> {
        Some(
            self.parameters
                .iter()
                .flat_map(|(name, values)| {
                    values
                        .iter()
                        .map(move |value| (name.as_str(), value.as_str()))
                })
                .collect(),
        )
    }

    fn parameter_values(&self, name: &str) -> Option<Vec<&str>> {
        self.parameters
            .get(name)
            .map(|values| values.iter().map(String::as_str).collect())
    }

    fn path(&self) -> &str {
        self.uri.path()
    }

    fn host(&self) -> Option<&str> {
        self.uri.host()
    }

    fn scheme(&self) -> Option<&str> {
        self.uri.scheme().map(|scheme| scheme.as_str())
    }

    fn server_port(&self) -> Option<u16> {
        self.uri.port_u16()
    }

    fn server_name(&self) -> Option<String> {
        self.request.server_name()
    }

    fn context_path(&self) -> Option<&str> {
        self.request.context_path()
    }

    fn locale(&self) -> Option<&Locale> {
        self.locales.first()
    }

    fn locales(&self) -> Option<Vec<&Locale>> {
        Some(self.locales.iter().collect())
    }

    fn get_attribute(&self, name: &str) -> Option<&next_web_core::anys::any_value::AnyValue> {
        self.request.get_attribute(name)
    }

    fn remove_attribute(&mut self, name: &str) {
        self.request.remove_attribute(name);
    }

    fn set_attribute(&mut self, name: &str, value: next_web_core::anys::any_value::AnyValue) {
        self.request.set_attribute(name, value);
    }

    fn ready(&mut self) {
        self.request.ready();
    }

    fn clean_up(&mut self) {
        self.request.clean_up();
    }

    fn is_secure(&self) -> bool {
        self.request.is_secure()
    }

    fn remote_addr(&self) -> Option<&std::net::SocketAddr> {
        self.request.remote_addr()
    }
}
