
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TencentCloudSmsResponse {
    Ok {
        #[serde(rename = "SendStatusSet")]
        send_status_set: Vec<SendStatusSet>,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
    Error {
        #[serde(rename = "Error")]
        error: ErrorRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,

    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ErrorRespnose {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct  SendStatusSet {
    /// 发送流水号
    pub serial_no: String,
    /// 手机号码，E.164标准，+[国家或地区码][手机号] ，示例如：+8618501234444， 其中前面有一个+号 ，86为国家码，18501234444为手机号
    pub phone_number: String,
    /// 计费条数，计费规则请查询 计费策略 https://cloud.tencent.com/document/product/382/36135
    pub fee: u32,
    /// 用户 session 内容
    pub session_context: String,
    /// 短信请求错误码，具体含义请参考 错误码，发送成功返回 "Ok"
    pub code: String,
    /// 短信请求错误码描述
    pub message: String,
    /// 国家码或地区码，例如 CN、US 等，对于未识别出国家码或者地区码，默认返回 DEF，具体支持列表请参考
    pub iso_code: String
}