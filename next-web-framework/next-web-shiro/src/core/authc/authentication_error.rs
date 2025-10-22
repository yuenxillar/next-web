#[derive(Debug)]
pub enum AuthenticationError {
    InvalidCredentials,
    AccountLocked,
    Unknown,

    Custom(String),
}