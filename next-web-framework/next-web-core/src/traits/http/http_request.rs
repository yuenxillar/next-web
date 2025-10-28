use std::str::FromStr;

use axum::{extract::Request, http::Uri};
use headers::{Cookie, HeaderMapExt};

use crate::{traits::http::request_dispatcher::RequestDispatcher, util::http_method::HttpMethod};
pub trait HttpRequest
where Self: Send
 {
    fn session(&self, name: &str) -> Option<String>;

    fn cookie(&self) -> Option<Cookie>;

    fn request_dispatcher(&self, default_failure_url: &str) -> Option<&dyn RequestDispatcher>;

    fn method(&self) -> HttpMethod;

    fn header(&self, header_name: &str) -> Option<&str>;

    fn uri(&self) -> &Uri;

    fn path(&self) -> &str;

    fn host(&self) -> Option<&str>;

    fn scheme(&self) -> Option<&str>;
}

impl HttpRequest for Request {
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

    fn header(&self, header_name: &str) -> Option<&str> {
        self.headers()
            .get(header_name)
            .map(|value| value.to_str().ok().unwrap_or_default())
    }
    
    fn uri(&self) -> &Uri {
        self.uri()
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
}
