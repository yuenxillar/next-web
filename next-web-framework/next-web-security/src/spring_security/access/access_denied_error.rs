use axum::Form;



pub struct AccessDeniedError {
    pub msg: String,
    cause: Option<Box<dyn std::error::Error>>,
}


impl From<&str> for AccessDeniedError {
    fn from(value: &str) -> Self {
        Self { msg: value.to_string(), cause: None }
    }
}