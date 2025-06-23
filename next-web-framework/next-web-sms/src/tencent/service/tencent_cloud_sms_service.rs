use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use next_web_core::{async_trait, core::service::Service, error::BoxError};
use reqwest::header::HeaderMap;

use crate::core::{service::sms_service::SmsSendService};

#[derive(Clone)]
pub struct TencentCloudSmsService {
    sms_client: reqwest::Client
}

impl Service for TencentCloudSmsService {}

const ENDPOINT: &'static str = "sms.tencentcloudapi.com";
const METHOD: &'static str = "POST";
const VERSION: &'static str = "2021-01-11";
const LANGUAGE: &'static str = "zh-CN";
const CONTENT_TYPE: &'static str = "application/json";
const REGION: &'static str = "ap-guangzhou";

#[async_trait]
impl SmsSendService for TencentCloudSmsService {
    type Response = ();

    /// read this doc: https://cloud.tencent.com/document/api/382
    async fn send_sms<'a>(
        &self,
        phone_numbers: &'a str,
        sign_name: &'a str,
        template_code: &'a str,
        template_param: &'a str,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> Result<Self::Response, BoxError> {
        let mut params = BTreeMap::new();

        // let phone_numbers = phone_numbers.join(",");
        params.insert("SignName", sign_name.into());
        params.insert("PhoneNumberSet", phone_numbers.into());
        params.insert("SmsSdkAppId", phone_numbers.into());

        params.insert("TemplateId", template_code.into());
        params.insert("TemplateParamSet", template_param.into());
        params.insert("SessionContext", template_param.into());

        expand_params.map(|var| params.extend(var));

        let body = serde_json::to_string(&params).unwrap();
        let mut common_req_headers = self.common_req_headers();
        common_req_headers.insert("Authorization", "".into());

        let resp = self.sms_client
            .post(self.url())
            .body(body)
            .headers(HeaderMap::new())
            .send()
            .await;
        Ok(())
    }


    async fn send_batch_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_names: Vec<&'a str>,
        template_code: &'a str,
        template_param: Vec<&'a str>,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> Result<Self::Response, BoxError> { Err("()".into())}


    fn check_validity<'a>(&self, phone_number: &'a str, sign_name: &'a str) -> bool { false }

    
    fn url(&self) -> &str {
        "https://sms.tencentcloudapi.com"
    }

    ///
    ///
    /// HTTP 请求头：X-TC-Timestamp。当前 UNIX 时间戳，可记录发起 API 请求的时间。例如 1529223702。注意：如果与服务器时间相差超过5分钟，会引起签名过期错误.
    ///
    ///	HTTP 请求头：X-TC-Version。操作的 API 的版本。取值参考接口文档中入参公共参数 Version 的说明。例如云服务器的版本 2017-03-12。
    ///
    /// HTTP 请求头：X-TC-Language。指定接口返回的语言，仅部分接口支持此参数。取值：zh-CN，en-US。zh-CN 返回中文，en-US 返回英文。
    fn common_req_headers(&self) -> BTreeMap<&str, String> {
        let mut params = BTreeMap::new();

        let unix_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();
        params.insert("X-TC-Timestamp", unix_timestamp);
        params.insert("X-TC-Version", VERSION.into());
        params.insert("X-TC-Language", LANGUAGE.into());
        params.insert("X-TC-Region", REGION.into());

        params.insert("Host", ENDPOINT.into());
        params.insert("Content-Type", CONTENT_TYPE.into());
        params
    }
}

mod test {

    use std::{collections::BTreeMap, time::Instant};

    use crate::signature::v3::SignerV3;

    #[test]
    fn test() {
        let signer = SignerV3::new("AKID", "SECRET", "cvm", "ap-guangzhou", "");

        let mut headers = BTreeMap::new();
        headers.insert("X-TC-Action", "SendSms".to_string());
        headers.insert("X-TC-Version", "2017-03-12".to_string());
        headers.insert("Content-Type", "application/json".to_string());

        let canonical = signer.sign(
            "POST",
            "/",
            &BTreeMap::new(),
            &headers,
            b"test666",
        );

        println!("{:?}", canonical);
    }
}
