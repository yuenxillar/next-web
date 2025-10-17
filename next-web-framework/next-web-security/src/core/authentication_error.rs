use std::{error::Error, fmt::Display};

use next_web_core::anys::{any_error::AnyError, any_value::AnyValue};

#[derive(Debug, Clone)]
pub struct AuthenticationError {
    msg: String,
    cause: Option<Box<dyn AnyError>>
}


impl AuthenticationError {

    pub fn get_message(&self) -> &str {
        &self.msg
    }
}
impl Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Authentication Error")
    }
}


impl Error for AuthenticationError {}

impl Into<AnyValue> for  AuthenticationError  {
    fn into(self) -> AnyValue {
        AnyValue::Object(Box::new(self))
    }
}