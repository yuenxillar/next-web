use std::{
    collections::BTreeMap, str::FromStr, sync::Arc
};

use next_web_core::{async_trait, core::service::Service, error::BoxError};
use once_cell::sync::Lazy;
use reqwest::{header::{HeaderMap, HeaderName}, Method};

use crate::{
    core::{service::sms_service::SmsSendService, signer::SignerV3},
    tencent::signer::TencentCloudSigner,
};

static SECRET_ID: Lazy<Arc<String>> =
    Lazy::new(|| Arc::new(std::env::var("TENCENTCLOUD_SECRET_ID").unwrap()));

static SECRET_KEY: Lazy<Arc<String>> =
    Lazy::new(|| Arc::new(std::env::var("TENCENTCLOUD_SECRET_KEY").unwrap()));

#[derive(Clone)]
pub struct TencentCloudSmsService {
    sms_client: reqwest::Client,
}

impl Service for TencentCloudSmsService {}

const SMS_ENDPOINT: &'static str = "sms.tencentcloudapi.com";
const SMS_VERSION: &'static str = "2021-01-11";
const LANGUAGE: &'static str = "zh-CN";
const PATH: &'static str = "/";
const ALGORITHM: &'static str = "TC3-HMAC-SHA256";
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

        params.insert("PhoneNumberSet", phone_numbers.into());
        params.insert("SignName", sign_name.into());

        params.insert("TemplateId", template_code.into());
        params.insert("TemplateParamSet", template_param.into());

        expand_params.map(|var| {
            let required = var.contains_key("SmsSdkAppId");
            params.extend(var);
            if !required { None } else { Some(()) }
        }).ok_or("expand_params missing SmsSdkAppId!")?;

        let body = serde_json::to_string(&params)?;
        let mut common_req_headers = self.common_req_headers();
        let signer = TencentCloudSigner { service: "cvm" };
        let authorization = signer.sign(
            Method::POST.as_str(),
            PATH,
            None,
            &common_req_headers,
            &body,
            SECRET_KEY.as_str(),
            SECRET_ID.as_str(),
            ALGORITHM,
        )?;
        common_req_headers.insert("Authorization", authorization);

        let headers = to_header_map(common_req_headers);
        let resp = self
            .sms_client
            .post(self.url())
            .body(body)
            .headers(headers)
            .send()
            .await?;

        println!("{:?}", resp.text().await);
        Ok(())
    }

    async fn send_batch_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_names: Vec<&'a str>,
        template_code: &'a str,
        template_param: Vec<&'a str>,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> Result<Self::Response, BoxError> {
        Err("()".into())
    }

    fn check_validity<'a>(&self, phone_number: &'a str, sign_name: &'a str) -> bool {
        false
    }

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
        let mut headers = BTreeMap::new();

        headers.insert("Host", SMS_ENDPOINT.into());
        headers.insert("Content-Type", CONTENT_TYPE.into());

        let unix_timestamp = chrono::Utc::now().timestamp();
        headers.insert("X-TC-Timestamp", unix_timestamp.to_string());
        headers.insert("X-TC-Version", SMS_VERSION.into());
        headers.insert("X-TC-Language", LANGUAGE.into());
        headers.insert("X-TC-Region", REGION.into());

        headers
    }
}

fn to_header_map(headers: BTreeMap<&str, String>) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    headers.into_iter().for_each(|(key, value)| {
        header_map.insert(HeaderName::from_str(&key).unwrap(), value.parse().unwrap());
    });

    header_map
}

#[cfg(test)]
mod tencent_sms_test {

    use std::{collections::BTreeMap, time::Instant};

    #[tokio::test]
    async fn test_send_sms() {

    }
}
