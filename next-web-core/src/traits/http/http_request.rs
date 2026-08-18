use crate::http::HttpVersion;
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, Uri, uri::Scheme},
};

use headers::{HeaderMapExt, Host};
use std::{collections::HashMap, net::SocketAddr};

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

    fn locale(&self) -> Option<&Locale>;

    fn locales(&self) -> Option<Vec<&Locale>>;

    fn get_attribute(&self, name: &str) -> Option<&AnyValue>;

    fn remove_attribute(&mut self, name: &str);

    fn set_attribute(&mut self, name: &str, value: AnyValue);

    fn ready(&mut self);

    fn clean_up(&mut self);

    fn is_secure(&self) -> bool;

    fn remote_addr(&self) -> Option<&SocketAddr>;
}

pub type OneMap = HashMap<String, AnyValue>;

impl HttpRequest for Request {
    fn auth_type(&self) -> AuthType {
        AuthType::from_request(self)
    }

    // fn session(&self, name: &str) -> Option<String> {
    //     self.cookie()
    //         .map(|cookie| cookie.get(name).map(ToString::to_string))
    //         .unwrap_or_default()
    // }

    fn session(&self) -> Option<&dyn HttpSession> {
        None
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

    fn requested_session_id(&self) -> Option<&str> {
        todo!()
    }

    fn cookie(&self) -> Option<&Cookie> {
        todo!()
    }

    fn cookies(&self) -> Option<&[Cookie]> {
        todo!()
    }

    fn request_dispatcher(&self, mut path: &str) -> Option<&dyn RequestDispatcher> {
        if path.is_empty() {
            return None;
        }

        let fragment_pos = path.find('#');
        if fragment_pos.is_some() {
            if let Some(var) = path.get(0..fragment_pos.unwrap()) {
                path = var;
            }
        }

        // If the path is already context-relative, just pass it through
        if path.starts_with('/') {
            // return Some(());
        }

        let request_path = self.path();
        let pos = request_path.rfind('/');
        let mut relative = None;
        if pos.is_some() {
            if let Some(data) = request_path.get(0..pos.unwrap() + 1) {
                let str1 = urlencoding::encode(data);
                let mut str2 = String::from(str1);
                str2.push_str(path);
                relative = Some(str2);
            }
        } else {
            relative = Some(String::from(urlencoding::encode(request_path)) + path);
        }

        // Validate the path argument

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
        self.query().and_then(|query| {
            query
                .split('&')
                .find_map(|param| param.split('=').nth(1).filter(|value| *value == name))
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

    fn locale(&self) -> Option<&Locale> {
        todo!()
    }

    fn locales(&self) -> Option<Vec<&Locale>> {
        todo!()
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
}
