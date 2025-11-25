use axum::{
    extract::Request,
    http::{uri::Scheme, Uri, Version},
};
use headers::{Cookie, HeaderMapExt, Host};
use std::{collections::HashMap, str::FromStr};

use crate::{
    anys::any_value::AnyValue, http::auth_type::AuthType,
    traits::http::request_dispatcher::RequestDispatcher, util::http_method::HttpMethod,
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

    fn request_dispatcher(&self, default_failure_url: &str) -> Option<&dyn RequestDispatcher> {
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
        self.get_attribute("serverContextPath")
            .map(|val| val.as_str())
            .unwrap_or_default()
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
        self.extensions_mut()
            .get_mut::<OneMap>()
            .map(|map| map.clear());

        self.extensions_mut().remove::<OneMap>();
    }

    fn is_secure(&self) -> bool {
        self.uri().scheme() == Some(&Scheme::HTTPS)
    }
}
