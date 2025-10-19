#[derive(Debug)]
pub enum AuthenticationError {
    InvalidCredentials,
    AccountLocked,
    Unknown,
    IllegalState(String)
}