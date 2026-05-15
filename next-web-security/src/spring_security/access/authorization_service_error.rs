#[derive(Debug)]
pub struct AuthorizationServiceError {
    pub msg: String,
    pub cause: Option<Box<dyn std::error::Error>>,
}

impl From<&str> for AuthorizationServiceError {
    fn from(value: &str) -> Self {
        Self {
            msg: value.to_string(),
            cause: None,
        }
    }
}

impl std::fmt::Display for AuthorizationServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for AuthorizationServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref())
    }
}
