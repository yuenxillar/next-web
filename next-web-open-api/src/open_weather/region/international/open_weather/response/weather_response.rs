use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum WeatherResponse<T> {
    Ok(T),
    Error(ErrorResponse),
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    /// Code of error
    ///
    /// 错误代码
    pub cod: u16,
    /// Description of error
    ///
    /// 错误描述
    pub message: String,

    /// (optional) List of request parameters names that are related to this particular error
    ///
    ///（可选） 与此特定错误相关的请求参数名称列表
    pub parameters: Option<Vec<String>>,
}
