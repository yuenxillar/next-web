use axum::http::header;
use axum::{
    body::{Body, Bytes},
    http::{HeaderName, StatusCode, Version},
    response::{IntoResponse, Response},
};

pub trait HttpResponse
where
    Self: IntoResponse,
    Self: Send,
{
    fn version(&self) -> Version;

    fn status_code(&self) -> StatusCode;

    fn set_status_code(&mut self, status_code: StatusCode);

    fn header(&self, name: &str) -> Option<&str>;

    fn append_header(&mut self, name: &[u8], value: &str) -> bool;

    fn insert_header(&mut self, name: &[u8], value: &str) -> Option<String>;

    fn remove_header(&mut self, name: &[u8]) -> Option<String>;

    fn set_body(&mut self, body: Bytes);

    fn set_redirect(&mut self, url: &str);

    // fn set_cookie(&mut self, cookie: Cookie);
}

impl HttpResponse for Response {
    fn version(&self) -> Version {
        self.version()
    }

    fn status_code(&self) -> StatusCode {
        self.status()
    }

    fn set_status_code(&mut self, status_code: StatusCode) {
        *self.status_mut() = status_code;
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers()
            .get(name)
            .map(|value| value.to_str().ok().unwrap_or_default())
    }

    fn append_header(&mut self, name: &[u8], value: &str) -> bool {
        let value = match value.parse() {
            Ok(s) => s,
            Err(_) => return false,
        };
        HeaderName::from_bytes(name)
            .map(|name| self.headers_mut().append(name, value))
            .ok()
            .unwrap_or_default()
    }

    fn insert_header(&mut self, name: &[u8], value: &str) -> Option<String> {
        let name = match HeaderName::from_bytes(name) {
            Ok(name) => name,
            Err(_) => return None,
        };

        value.parse().ok().map(|value| {
            self.headers_mut()
                .insert(name, value)
                .map(|s| s.to_str().ok().map(ToString::to_string).unwrap_or_default())
                .unwrap_or_default()
        })
    }

    fn remove_header(&mut self, name: &[u8]) -> Option<String> {
        HeaderName::from_bytes(name)
            .map(|name| self.headers_mut().remove(name))
            .ok()
            .flatten()
            .map(|s| s.to_str().ok().map(ToString::to_string).unwrap_or_default())
    }

    fn set_body(&mut self, body: Bytes) {
        *self.body_mut() = Body::from(body);
    }

    fn set_redirect(&mut self, url: &str) {
        if let Ok(url) = header::HeaderValue::from_str(url) {
            *self.status_mut() = StatusCode::SEE_OTHER;
            self.headers_mut().insert(header::LOCATION, url);
        }
    }
}
