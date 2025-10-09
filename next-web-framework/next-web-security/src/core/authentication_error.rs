use std::{error::Error, fmt::Display};

use next_web_core::models::any_error::AnyError;



#[derive(Debug, Clone)]
pub struct AuthenticationError {
    msg: String,
    cause: Option<Box<dyn AnyError>>
}

impl Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Authentication Error")
    }
}


impl Error for AuthenticationError {}