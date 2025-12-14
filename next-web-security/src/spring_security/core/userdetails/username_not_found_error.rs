
#[derive(Debug, Clone)]
pub struct UsernameNotFoundError(pub String);

impl std::error::Error for UsernameNotFoundError {}

impl std::fmt::Display for UsernameNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Username not found: {}", self.0)
    }
}