#[derive(Debug)]
pub enum AuthenticationError {
    InvalidCredentials,
    AccountLocked,
    Unknown,
    NotImplemented(String),

    Custom(String),
}
