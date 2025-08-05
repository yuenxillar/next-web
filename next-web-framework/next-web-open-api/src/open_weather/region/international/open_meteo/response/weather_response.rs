use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum WeatherResponse<T> {
    Ok(T),
    Error(ErrorResponse),
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    /// Error true
    ///
    /// 错误 true
    pub error: bool,
    /// Reason for error
    ///
    /// 错误原因
    pub reason: Box<str>,
}
