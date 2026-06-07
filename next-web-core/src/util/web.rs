use crate::{http::cookie::Cookie, traits::http::http_request::HttpRequest};

pub struct WebUtils;

impl WebUtils {
    pub fn get_cookie<'a>(request: &'a dyn HttpRequest, name: &str) -> Option<&'a Cookie> {
        if let Some(cookies) = request.cookies() {
            for cookie in cookies {
                if cookie.name() == name {
                    return Some(cookie);
                }
            }
        }

        None
    }
}
