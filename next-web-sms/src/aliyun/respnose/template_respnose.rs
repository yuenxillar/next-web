#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AliyunCloudTemplateResponse<T> {
    /// 请求 ID
    pub request_id: String,
    /// 状态码
    pub code: String,
    /// 状态码的描述。
    pub message: String,
    /// 拓展
    #[serde(flatten)]
    pub params: Option<T>,
}
