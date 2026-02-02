use axum::{
    extract::Request,
    http::{uri::Scheme, Uri, Version},
};
use headers::{Cookie, HeaderMapExt, Host};
use std::{collections::HashMap, str::FromStr};

use crate::{
    anys::any_value::AnyValue, autoconfigure::context::server_properties::GLOBAL_SERVER_PROPERTIES,
    http::auth_type::AuthType, traits::http::request_dispatcher::RequestDispatcher,
    util::http_method::HttpMethod,
};

pub const IDENTITY_REMOVED_KEY: &str = stringify!(format!(
    "{}_IDENTITY_REMOVED_KEY",
    std::any::type_name::<HttpRequest>()
));

pub trait HttpRequest
where
    Self: Send,
{
    fn session(&self, name: &str) -> Option<String>;

    fn auth_type(&self) -> AuthType;

    fn cookie(&self) -> Option<Cookie>;

    fn request_dispatcher(&self, default_failure_url: &str) -> Option<&dyn RequestDispatcher>;

    fn method(&self) -> HttpMethod;

    fn version(&self) -> Version;

    fn header(&self, header_name: &str) -> Option<&str>;

    fn uri(&self) -> &Uri;

    fn query(&self) -> Option<&str>;

    fn get_parameter(&self, name: &str) -> Option<&str>;

    fn path(&self) -> &str;

    fn host(&self) -> Option<&str>;

    fn scheme(&self) -> Option<&str>;

    fn server_port(&self) -> Option<u16>;

    fn server_name(&self) -> Option<String>;

    fn context_path(&self) -> Option<&str>;

    fn get_attribute(&self, name: &str) -> Option<&AnyValue>;

    fn remove_attribute(&mut self, name: &str);

    fn set_attribute(&mut self, name: &str, value: AnyValue);

    fn ready(&mut self);

    fn clean_up(&mut self);

    fn is_secure(&self) -> bool;
}

pub type OneMap = HashMap<String, AnyValue>;

impl HttpRequest for Request {
    fn auth_type(&self) -> AuthType {
        AuthType::from_request(self)
    }

    fn session(&self, name: &str) -> Option<String> {
        self.cookie()
            .map(|cookie| cookie.get(name).map(ToString::to_string))
            .unwrap_or_default()
    }

    fn cookie(&self) -> Option<Cookie> {
        self.headers().typed_get::<Cookie>()
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
        HttpMethod::from_str(self.method().as_str()).unwrap_or_default()
    }

    fn version(&self) -> Version {
        self.version()
    }

    fn header(&self, header_name: &str) -> Option<&str> {
        self.headers()
            .get(header_name)
            .map(|value| value.to_str().ok().unwrap_or_default())
    }

    fn uri(&self) -> &Uri {
        self.uri()
    }

    fn query(&self) -> Option<&str> {
        self.uri().query()
    }

    fn get_parameter(&self, name: &str) -> Option<&str> {
        self.query().and_then(|query| {
            query
                .split('&')
                .find_map(|param| param.split('=').nth(1).filter(|value| *value == name))
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
}
