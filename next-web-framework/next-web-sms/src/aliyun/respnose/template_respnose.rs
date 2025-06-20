#[derive(Clone, Debug, serde::Deserialize)]
pub struct TemplateResponse<T>
{
    /// 请求 ID。
    #[serde(rename = "RequestId")]
    pub request_id: String,
    #[serde(rename = "Code")]
    pub code: String,
    /// 状态码的描述。
    #[serde(rename = "Message")]
    pub message: String,
    // 拓展参数
    #[serde(flatten)]
    pub params: T,
}
