use std::{error::Error, fmt::Display};

#[derive(Debug, PartialEq, Eq)]
pub enum AuthorizationError {
    Unauthorized(String),
    Forbidden(String),
}

impl Error for AuthorizationError {}

impl Display for AuthorizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthorizationError::Unauthorized(_) => todo!(),
            AuthorizationError::Forbidden(_) => todo!(),
        }
    }
}