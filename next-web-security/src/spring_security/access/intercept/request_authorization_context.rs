use axum::extract::Request;

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

    pub fn from_request(request: &Request) -> Self {
        Self::new(request.uri().path(), request.method().as_str())
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn method(&self) -> &str {
        &self.method
    }
}
