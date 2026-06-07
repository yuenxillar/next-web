use next_web_core::traits::http::http_request::HttpRequest;

#[derive(Clone, Default)]
pub struct RequestAuthorizationContext {
    path: String,
    method: String,
}

impl RequestAuthorizationContext {
    pub fn new(path: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            method: method.into(),
        }
    }

    pub fn from_request(request: &dyn HttpRequest) -> Self {
        Self::new(request.path(), request.method().to_string())
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn method(&self) -> &str {
        &self.method
    }
}
