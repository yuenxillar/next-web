use crate::http::{HttpRequestShare, HttpVersion};
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, Uri, header::CONTENT_TYPE, uri::Scheme},
};

use headers::{Cookie as HeaderCookie, HeaderMapExt, Host};
use std::{collections::HashMap, net::SocketAddr, sync::OnceLock};

use crate::{
    anys::any_value::AnyValue,
    autoconfigure::context::server_properties::GLOBAL_SERVER_PROPERTIES,
    http::{Cookie, HttpMethod, auth_type::AuthType},
    traits::http::{HttpSession, request_dispatcher::RequestDispatcher},
    util::locale::Locale,
};

pub const IDENTITY_REMOVED_KEY: &str = stringify!(format!(
    "{}_IDENTITY_REMOVED_KEY",
    std::any::type_name::<HttpRequest>()
));

pub trait HttpRequest
where
    Self: Send,
{
    fn session(&self) -> Option<&dyn HttpSession>;

    fn session_mut(&mut self, create: bool) -> Option<&mut dyn HttpSession>;

    fn change_session_id(&mut self) -> String;

    fn is_requested_session_id_valid(&self) -> bool;

    fn requested_session_id(&self) -> Option<&str>;

    fn auth_type(&self) -> AuthType;

    fn cookie(&self) -> Option<&Cookie>;

    fn cookies(&self) -> Option<&[Cookie]>;

    fn request_dispatcher(&self, default_failure_url: &str) -> Option<&dyn RequestDispatcher>;

    fn method(&self) -> HttpMethod;

    fn version(&self) -> HttpVersion;

    fn headers(&self) -> &HeaderMap<HeaderValue>;

    fn header(&self, header_name: &str) -> Option<&str>;

    fn header_values(&self, header_name: &str) -> Vec<&str>;

    fn header_names(&self) -> Vec<&str>;

    fn uri(&self) -> &Uri;

    fn query(&self) -> Option<&str>;

    fn parameter(&self, name: &str) -> Option<&str>;

    fn parameters(&self) -> Option<Vec<(&str, &str)>>;

    fn parameter_values(&self, name: &str) -> Option<Vec<&str>>;

    fn path(&self) -> &str;

    fn host(&self) -> Option<&str>;

    fn scheme(&self) -> Option<&str>;

    fn server_port(&self) -> Option<u16>;

    fn server_name(&self) -> Option<String>;

    fn context_path(&self) -> Option<&str>;

    fn locale(&self) -> Option<Locale>;

    fn locales(&self) -> Option<Vec<Locale>>;

    fn content_type(&self) -> Option<&str>;

    fn get_attribute(&self, name: &str) -> Option<&AnyValue>;

    fn remove_attribute(&mut self, name: &str);

    fn set_attribute(&mut self, name: &str, value: AnyValue);

    fn ready(&mut self);

    fn clean_up(&mut self);

    fn is_secure(&self) -> bool;

    fn remote_addr(&self) -> Option<&SocketAddr>;

    fn shared(&mut self) -> &HttpRequestShare;
}

pub type OneMap = HashMap<String, AnyValue>;

impl HttpRequest for Request {
    fn auth_type(&self) -> AuthType {
        AuthType::from_request(self)
    }

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
        let lock = self.extensions().get::<OnceLock<Vec<Cookie>>>()?;
        Some(lock.get_or_init(|| parse_cookies(self)).as_slice())
    }

    fn request_dispatcher(&self, _default_failure_url: &str) -> Option<&dyn RequestDispatcher> {
        None
    }

    fn method(&self) -> HttpMethod {
        self.method().to_owned()
    }

    fn version(&self) -> HttpVersion {
        self.version()
    }

    fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.headers()
    }

    fn content_type(&self) -> Option<&str> {
        self.headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
    }

    fn header(&self, header_name: &str) -> Option<&str> {
        self.headers()
            .get(header_name)
            .map(|value| value.to_str().ok().unwrap_or_default())
    }

    fn header_values(&self, header_name: &str) -> Vec<&str> {
        self.headers()
            .get_all(header_name)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect()
    }

    fn header_names(&self) -> Vec<&str> {
        self.headers()
            .keys()
            .into_iter()
            .map(|k| k.as_str())
            .collect()
    }

    fn uri(&self) -> &Uri {
        self.uri()
    }

    fn query(&self) -> Option<&str> {
        self.uri().query()
    }

    fn parameter(&self, name: &str) -> Option<&str> {
        let query = self.query()?;
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key == name).then_some(value)
        })
    }

    fn parameters(&self) -> Option<Vec<(&str, &str)>> {
        self.query().map(|query| {
            query
                .split('&')
                .filter(|s| !s.is_empty())
                .map(|param| {
                    let mut parts = param.splitn(2, '=');
                    let key = parts.next().unwrap_or("");
                    let value = parts.next().unwrap_or("");
                    (key, value)
                })
                .collect()
        })
    }

    fn parameter_values(&self, name: &str) -> Option<Vec<&str>> {
        self.parameters().map(|params| {
            params
                .into_iter()
                .filter(|(key, _)| *key == name)
                .map(|(_, value)| value)
                .collect()
        })
    }

    fn path(&self) -> &str {
        self.uri().path()
    }

    fn host(&self) -> Option<&str> {
        self.uri().host()
    }

    fn scheme(&self) -> Option<&str> {
        self.uri().scheme().map(|s| s.as_str())
    }

    fn server_port(&self) -> Option<u16> {
        self.uri().port_u16()
    }

    fn server_name(&self) -> Option<String> {
        self.headers()
            .typed_get::<Host>()
            .map(|host| host.hostname().to_string())
    }

    fn context_path(&self) -> Option<&str> {
        GLOBAL_SERVER_PROPERTIES
            .get()
            .map(|var| var.context_path())?
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

    fn remove_attribute(&mut self, name: &str) {
        if let Some(map) = self.extensions_mut().get_mut::<OneMap>() {
            map.remove(name);
        }
    }

    fn set_attribute(&mut self, name: &str, value: AnyValue) {
        if let Some(map) = self.extensions_mut().get_mut::<OneMap>() {
            map.insert(name.to_string(), value);
        }
    }

    fn get_attribute(&self, name: &str) -> Option<&AnyValue> {
        self.extensions()
            .get::<OneMap>()
            .map(|map| map.get(name))
            .unwrap_or_default()
    }

    fn ready(&mut self) {
        self.extensions_mut().insert(OneMap::new());
        self.extensions_mut().insert(OnceLock::<Vec<Cookie>>::new());
    }

    fn clean_up(&mut self) {
        self.extensions_mut().remove::<OneMap>();
    }

    fn is_secure(&self) -> bool {
        self.uri().scheme() == Some(&Scheme::HTTPS)
    }

    fn remote_addr(&self) -> Option<&SocketAddr> {
        self.extensions().get::<SocketAddr>()
    }

    fn shared(&mut self) -> &HttpRequestShare {
        let has_shared = self.extensions().get::<HttpRequestShare>().is_some();

        if !has_shared {
            let shared = HttpRequestShare::from(self as &dyn HttpRequest);
            self.extensions_mut().insert(shared);
        }

        self.extensions()
            .get::<HttpRequestShare>()
            .expect("HttpRequestShare not found")
    }
}

fn parse_cookies(request: &Request) -> Vec<Cookie> {
    let Some(cookie_header) = request.headers().typed_get::<HeaderCookie>() else {
        return Vec::new();
    };
    cookie_header
        .iter()
        .map(|(name, value)| Cookie::new(name, Some(value.to_string())))
        .collect()
}
