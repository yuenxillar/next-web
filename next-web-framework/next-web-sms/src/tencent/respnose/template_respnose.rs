use serde::Deserialize;

use crate::tencent::respnose::sms_respnose::ErrorRespnose;


#[derive(Debug, Deserialize)]
pub struct TencentCloudTemplateResponse {
    #[serde(rename = "Response")]
    pub response: TemplateResponse,
}


#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TemplateResponse {
    Ok(crate::tencent::models::sms_template_respnose::Respnose),
    Error {
        #[serde(rename = "Error")]
        error: ErrorRespnose,
        #[serde(rename = "RequestId")]
        request_id: String,
    },
}