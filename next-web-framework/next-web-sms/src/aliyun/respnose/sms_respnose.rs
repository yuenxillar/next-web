#[derive(Clone, Debug, serde::Deserialize)]
pub struct AliyunCloudSmsResponse {
    #[serde(rename = "Code")]
    pub code: String,
    /// 状态码的描述。
    #[serde(rename = "Message")]
    pub message: String,
    /// 发送回执 ID。
    /// 可根据发送回执 ID 在接口 QuerySendDetails 中查询具体的发送状态。
    #[serde(rename = "BizId")]
    pub buz_id: String,
    /// 请求 ID。
    #[serde(rename = "RequestId")]
    pub request_id: String,
}
