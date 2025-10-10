use axum::extract::Request;
use headers::{Cookie, HeaderMapExt};

use crate::traits::http::request_dispatcher::RequestDispatcher;
pub trait HttpRequest {
    fn get_session(&self, name: &str) -> Option<String>;

    fn get_cookie(&self) -> Option<Cookie>;

    fn get_request_dispatcher(&self, default_failure_url: &str) -> Option<&dyn RequestDispatcher>;
}

impl HttpRequest for Request {
    fn get_session(&self, name: &str) -> Option<String> {
        self.get_cookie()
            .map(|cookie| cookie.get(name).map(ToString::to_string))
            .unwrap_or_default()
    }

    fn get_cookie(&self) -> Option<Cookie> {
        self.headers().typed_get::<Cookie>()
    }

    fn get_request_dispatcher(&self, default_failure_url: &str) -> Option<&dyn RequestDispatcher> {
        None
    }
}
