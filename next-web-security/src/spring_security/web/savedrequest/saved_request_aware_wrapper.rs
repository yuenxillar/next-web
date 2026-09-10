use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;
use next_web_core::http::{HeaderMap, HeaderValue, HttpRequestShare, Uri};
use next_web_core::traits::http::request_dispatcher::RequestDispatcher;
use next_web_core::traits::http::HttpSession;
use next_web_core::{
    http::{header::CONTENT_TYPE, Cookie, HttpMethod},
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
    saved_request: Arc<dyn SavedRequest>,
}

impl<'a> SavedRequestAwareWrapper<'a> {
    /// Creates a new instance that reads parameters, headers and cookies from the saved
    /// request and delegates the remaining state to the wrapped request.
    ///
    /// # Arguments
    ///
    /// * `saved` - the saved request to source parameters, headers and cookies from
    /// * `request` - the original request being wrapped
    pub fn new(saved_request: Arc<dyn SavedRequest>, request: &'a mut dyn HttpRequest) -> Self {
        Self {
            request,
            saved_request,
        }
    }
}

impl HttpRequest for SavedRequestAwareWrapper<'_> {
    fn session(&self) -> Option<&dyn HttpSession> {
        self.request.session()
    }

    fn session_mut(&mut self, create: bool) -> Option<&mut dyn HttpSession> {
        self.request.session_mut(create)
    }

    fn change_session_id(&mut self) -> String {
        self.request.change_session_id()
    }

    fn is_requested_session_id_valid(&self) -> bool {
        self.request.is_requested_session_id_valid()
    }

    fn requested_session_id(&self) -> Option<&str> {
        self.request.requested_session_id()
    }

    fn auth_type(&self) -> next_web_core::http::auth_type::AuthType {
        self.request.auth_type()
    }

    fn cookie(&self) -> Option<&Cookie> {
        self.request.cookie()
    }

    fn cookies(&self) -> Option<&[Cookie]> {
        self.request.cookies()
    }

    fn request_dispatcher(&self, default_failure_url: &str) -> Option<&dyn RequestDispatcher> {
        self.request.request_dispatcher(default_failure_url)
    }

    fn method(&self) -> HttpMethod {
        HttpMethod::from_str(self.saved_request.get_method()).unwrap_or(HttpMethod::default())
    }

    fn version(&self) -> next_web_core::http::HttpVersion {
        self.request.version()
    }

    fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.request.headers()
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.saved_request
            .get_header_values(name)
            .first()
            .map(|s| *s)
    }

    fn header_values(&self, name: &str) -> Vec<&str> {
        self.saved_request.get_header_values(name)
    }

    fn header_names(&self) -> Vec<&str> {
        self.saved_request.get_header_names()
    }

    fn uri(&self) -> &Uri {
        self.request.uri()
    }

    fn query(&self) -> Option<&str> {
        self.request.query()
    }

    fn parameter(&self, name: &str) -> Option<&str> {
        self.request.parameter(name).or_else(|| {
            self.saved_request
                .get_parameter_values(name)
                .and_then(|v| v.first().map(|s| *s))
        })
    }

    fn parameters(&self) -> Option<Vec<(Cow<'_, str>, Cow<'_, str>)>> {
        self.request.parameters()
    }

    fn parameter_values(&self, name: &str) -> Option<Vec<Cow<'_, str>>> {
        let saved_request_params = self.saved_request.get_parameter_values(name);
        let wrapped_request_params = self.request.parameter_values(name);
        let saved_request_params = match saved_request_params {
            Some(params) => params.into_iter().map(|s| Cow::Borrowed(s)).collect(),
            None => return wrapped_request_params,
        };

        let wrapped_request_params = match wrapped_request_params {
            Some(params) => params,
            None => return Some(saved_request_params),
        };

        // We want to add all parameters of the saved request *apart from* duplicates of
        // those already added
        Some(
            saved_request_params
                .into_iter()
                .filter(|s| !wrapped_request_params.contains(&s))
                .collect(),
        )
    }

    fn content_type(&self) -> Option<&str> {
        self.header(CONTENT_TYPE.as_str())
    }

    fn path(&self) -> &str {
        self.request.path()
    }

    fn host(&self) -> Option<&str> {
        self.request.host()
    }

    fn scheme(&self) -> Option<&str> {
        self.request.scheme()
    }

    fn server_port(&self) -> Option<u16> {
        self.request.server_port()
    }

    fn server_name(&self) -> Option<String> {
        self.request.server_name()
    }

    fn context_path(&self) -> Option<&str> {
        self.request.context_path()
    }

    fn locale(&self) -> Option<Locale> {
        Some(
            self.saved_request
                .get_locales()
                .first()
                .map(Clone::clone)
                .unwrap_or(Locale::default()),
        )
    }

    fn locales(&self) -> Option<Vec<Locale>> {
        let mut locales = self.saved_request.get_locales();
        if locales.is_empty() {
            locales.push(Locale::default());
        }

        Some(locales)
    }

    fn get_attribute(&self, name: &str) -> Option<&AnyValue> {
        self.request.get_attribute(name)
    }

    fn remove_attribute(&mut self, name: &str) {
        self.request.remove_attribute(name);
    }

    fn set_attribute(&mut self, name: &str, value: AnyValue) {
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

    fn shared(&mut self) -> &HttpRequestShare {
        self.request.shared()
    }
}
