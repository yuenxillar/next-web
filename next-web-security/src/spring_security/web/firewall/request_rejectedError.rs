#[derive(Debug, Clone)]
pub struct RequestRejectedError(pub String);

impl std::fmt::Display for RequestRejectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Request rejected: {}", self.0)
    }
}

impl std::error::Error for RequestRejectedError {}