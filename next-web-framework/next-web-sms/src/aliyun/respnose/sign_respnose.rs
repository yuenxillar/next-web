#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AliyunCloudSignResponse<T> {
    /// 请求 ID
    pub request_id: String,
    /// 状态码
    pub code: String,
    /// 状态码的描述。
    pub message: String,
    /// expand_params
    /// 拓展参数
    #[serde(flatten)]
    pub params: Option<T>,
}
