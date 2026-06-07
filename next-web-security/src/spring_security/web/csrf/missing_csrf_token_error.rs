use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub struct MissingCsrfTokenError(pub String);

impl Error for MissingCsrfTokenError {}

impl Display for MissingCsrfTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MissingCsrfTokenError: Could not verify the provided CSRF token because no token was found to compare.")
    }
}
