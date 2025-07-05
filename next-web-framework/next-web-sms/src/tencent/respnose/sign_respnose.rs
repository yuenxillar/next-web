use serde::Deserialize;

use crate::tencent::respnose::sms_respnose::ErrorRespnose;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TencentCloudSignResponse {
    #[serde(rename = "Response")]
    pub response: SignResponse,
}


#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum SignResponse {
    Ok(crate::tencent::models::sms_sign_respnose::Respnose),
    Error {
        #[serde(rename = "Error")]
        error: ErrorRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
}