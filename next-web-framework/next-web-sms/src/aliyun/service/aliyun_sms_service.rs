use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::SystemTime};

use chrono::DateTime;
use next_web_core::{async_trait, core::service::Service, error::BoxError};
use once_cell::sync::Lazy;
use reqwest::{
    header::{HeaderMap, HeaderName},
    Method,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    aliyun::{respnose::sms_respnose::AliyunCloudSmsResponse, signer::AliyunSigner},
    core::{service::sms_service::SmsService, signer::SignerV3},
};

#[cfg(feature = "template")]
use crate::core::service::template_service::{TemplateResult, TemplateService};

const PATH: &'static str = "/";
const ENDPOINT: &'static str = "dysmsapi.aliyuncs.com";
const VERSION: &'static str = "2017-05-25";
const ALGORITHM: &'static str = "ACS3-HMAC-SHA256";
const EMPTY_BODY_HEX_HASH_256: &'static str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

static ALIBABA_CLOUD_ACCESS_KEY_ID: Lazy<Arc<String>> =
    Lazy::new(|| Arc::new(std::env::var("ALIBABA_CLOUD_ACCESS_KEY_ID").unwrap()));

static ALIBABA_CLOUD_ACCESS_KEY_SECRET: Lazy<Arc<String>> =
    Lazy::new(|| Arc::new(std::env::var("ALIBABA_CLOUD_ACCESS_KEY_SECRET").unwrap()));

#[derive(Clone)]
pub struct AliyunSmsService {
    sms_client: reqwest::Client,
}

impl AliyunSmsService {
    pub fn new() -> Self {
        Self {
            sms_client: reqwest::Client::new(),
        }
    }

    async fn call_api<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        query_params: &BTreeMap<&str, Value>,
        action: &str,
        req_body: &str,
        mut common_req_headers: BTreeMap<&str, String>,
    ) -> Result<T, BoxError> {
        common_req_headers.insert("x-acs-action", action.into());
        common_req_headers.insert("x-acs-content-sha256", EMPTY_BODY_HEX_HASH_256.into());

        let query_params: BTreeMap<&str, String>  = query_params.iter().map(|(k, v)| (*k, v.to_string())).collect();
        // build SignerV3
        let signer = AliyunSigner {};
         let authorization = signer.sign(
            method,
            path,
            Some(&query_params),
            &common_req_headers,
            req_body,
            ALIBABA_CLOUD_ACCESS_KEY_SECRET.as_str(),
            ALIBABA_CLOUD_ACCESS_KEY_ID.as_str(),
            ALGORITHM,
        )?;
        common_req_headers.insert("Authorization", authorization);

        let headers = to_header_map(common_req_headers);


        let canonical_query_string = build_sored_encoded_query_string(&query_params);
        let url = format!("{}{}?{}", self.url(), path, canonical_query_string);

        let resp = self.sms_client.post(url).headers(headers).send().await?;
        Ok(resp.json::<T>().await?)
    }
}

impl Service for AliyunSmsService {}

#[async_trait]
impl SmsService for AliyunSmsService {
    type Response = AliyunCloudSmsResponse;

    /// read this doc: https://next.api.aliyun.com/document/Dysmsapi/2017-05-25/SendSms
    async fn send_sms<'a>(
        &self,
        phone_numbers: &'a str,
        sign_name: &'a str,
        template_code: &'a str,
        template_param: &'a str,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<Self::Response, BoxError> {
        if !self.check_validity(phone_numbers, sign_name) {
            return Err("phone_numbers or sign_names is invalid.".into());
        }

        let action = "SendSms";

        let mut query_params: BTreeMap<&str, Value> = BTreeMap::new();

        query_params.insert("PhoneNumbers", phone_numbers.into());
        query_params.insert("SignName", sign_name.into());
        query_params.insert("TemplateCode", template_code.into());
        query_params.insert("TemplateParam", template_param.into());

        expand_params.map(|var| query_params.extend(var));

        let req_body = "";
        let common_req_headers = self.common_req_headers();

        self.call_api::<Self::Response>(
            Method::POST.as_str(),
            PATH,
            &query_params,
            action,
            req_body,
            common_req_headers,
        )
        .await
    }

    /// Send SMS messages to multiple phone numbers in batch.
    ///
    /// # Arguments
    ///
    /// * `phone_numbers` - List of phone numbers.
    /// * `sign_names` - Corresponding list of signature names.
    /// * `template_code` - The same template code used for all messages.
    /// * `template_params` - Parameters for each message in JSON array format.
    /// * `expand_params` - Optional additional request parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(AliyunCloudSmsResponse)` if successful.
    /// * `Err(BoxError)` if input validation or API call fails.
    async fn send_batch_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_names: Vec<&'a str>,
        template_code: &'a str,
        template_params: Vec<&'a str>,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<Self::Response, BoxError> {
        if phone_numbers.len() == 0 || sign_names.len() == 0 {
            return Err("phone_numbers or sign_names is empty.".into());
        }

        if phone_numbers.len() != sign_names.len() {
            return Err("phone_numbers and sign_names len not equal.".into());
        }

        if !phone_numbers.iter().enumerate().all(|(index, item)| {
            sign_names
                .get(index)
                .map(|sign| self.check_validity(&item, sign))
                .unwrap_or(false)
        }) {
            return Err("phone_numbers or sign_names is invalid.".into());
        }

        if template_params.len() != phone_numbers.len() {
            return Err("template_params and phone_numbers len not equal.".into());
        }

        let action = "SendBatchSms";

        let mut query_params: BTreeMap<&str, Value> = BTreeMap::new();

        query_params.insert("PhoneNumberJson", phone_numbers.into());
        query_params.insert("SignNameJson", sign_names.into());
        query_params.insert("TemplateCode", template_code.into());
        query_params.insert("TemplateParamJson", template_params.into());
        expand_params.map(|var| query_params.extend(var));

        let req_body = "";
        let common_req_headers = self.common_req_headers();

        self.call_api::<Self::Response>(
            Method::POST.as_str(),
            PATH,
            &query_params,
            action,
            req_body,
            common_req_headers,
        )
        .await
    }

    /// Validate phone number and signature name are non-empty.
    fn check_validity<'a>(&self, phone_number: &'a str, sign_name: &'a str) -> bool {
        // 检查手机号码是否合法
        if phone_number.trim_end().is_empty() {
            return false;
        }

        // 检查签名是否合法
        if sign_name.trim_end().is_empty() {
            return false;
        }

        true
    }
    fn url(&self) -> &str {
        "https://dysmsapi.aliyuncs.com"
    }

    fn common_req_headers(&self) -> BTreeMap<&str, String> {
        let mut headers = BTreeMap::new();

        headers.insert("Host", ENDPOINT.into());
        headers.insert("x-acs-version", VERSION.into());

        let now_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let datetime = DateTime::from_timestamp(now_time as i64, 0).unwrap_or_default();
        let date = datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        headers.insert("x-acs-date", date);
        headers.insert("x-acs-signature-nonce", uuid::Uuid::new_v4().to_string());

        headers
    }
}

// #[cfg(feature = "template")]
#[async_trait]
impl TemplateService for AliyunSmsService {
    async fn create_template<'a, R>(
        &self,
        template_name: &'a str,
        template_content: &'a str,
        template_type: i32,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> TemplateResult<R>
    where
        R: DeserializeOwned,
    {
        if template_name.is_empty() || template_content.is_empty() {
            return Err("template_name, template_content cannot be empty!".into());
        }

        if template_type == 0 {
            if !expand_params
                .as_ref()
                .map(|map| map.contains_key("RelatedSignName"))
                .unwrap_or(false)
            {
                return Err("RelatedSignName is required, Please input to expand_params!".into());
            }
        } else if template_type == 3 {
            if !expand_params
                .as_ref()
                .map(|map| map.contains_key("IntlType"))
                .unwrap_or(false)
            {
                return Err("IntlType is required, Please input to expand_params!".into());
            }
        }

        let action: &str = "CreateSmsTemplate";

        let mut query_params: BTreeMap<&str, Value> = BTreeMap::new();
        query_params.insert("TemplateName", template_name.into());
        query_params.insert("TemplateContent", template_content.into());
        query_params.insert("TemplateType", template_type.into());

        expand_params.map(|var| query_params.extend(var));

        let req_body = "";
        let common_req_headers = self.common_req_headers();

        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            &query_params,
            action,
            req_body,
            common_req_headers,
        )
        .await
    }

    async fn delete_template<R>(&self, template_code: &str) -> TemplateResult<R>
    where
        R: DeserializeOwned,
    {
        if template_code.is_empty() {
            return Err("template_code is empty!".into());
        }

        let action: &str = "DeleteSmsTemplate";

        let mut query_params: BTreeMap<&str, Value> = BTreeMap::new();
        query_params.insert("TemplateCode", urlencoding::encode(template_code).into());

        let req_body = "";
        let common_req_headers = self.common_req_headers();

        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            &query_params,
            action,
            req_body,
            common_req_headers,
        )
        .await
    }

    async fn update_template<'a, R>(
        &self,
        template_code: &'a str,
        template_name: &'a str,
        template_content: &'a str,
        template_type: i32,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> TemplateResult<R>
    where
        R: DeserializeOwned,
    {
        if template_code.is_empty() || template_name.is_empty() || template_content.is_empty() {
            return Err("template_code, template_name, template_content cannot be empty!".into());
        }

        if template_type == 0 {
            if !expand_params
                .as_ref()
                .map(|map| map.contains_key("RelatedSignName"))
                .unwrap_or(false)
            {
                return Err("RelatedSignName is required, Please input to expand_params!".into());
            }
        } else if template_type == 3 {
            if !expand_params
                .as_ref()
                .map(|map| map.contains_key("IntlType"))
                .unwrap_or(false)
            {
                return Err("IntlType is required, Please input to expand_params!".into());
            }
        }

        let action = "UpdateSmsTemplate";

        let mut query_params: BTreeMap<&str, Value> = BTreeMap::new();
        query_params.insert("TemplateName", template_name.into());
        query_params.insert("TemplateCode", template_code.into());
        query_params.insert("TemplateContent", template_content.into());
        query_params.insert("TemplateType", template_type.into());

        expand_params.map(|var| query_params.extend(var));

        let req_body = "";
        let common_req_headers = self.common_req_headers();

        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            &query_params,
            action,
            req_body,
            common_req_headers,
        )
        .await
    }

    async fn query_template<R>(
        &self,
        _template_type: i32,
        index: u16,
        size: u16,
    ) -> TemplateResult<R>
    where
        R: DeserializeOwned,
    {
        if index < 1 {
            return Err("PageIndex must be greater than 0".into());
        }

        if size <= 0 || size > 50 {
            return Err("PageSize must be greater than 0 and less than 50".into());
        }

        let action: &str = "QuerySmsTemplateList";

        let mut query_params: BTreeMap<&str, Value> = BTreeMap::new();
        query_params.insert("PageIndex", index.into());
        query_params.insert("PageSize", size.into());

        let req_body = "";
        let common_req_headers = self.common_req_headers();

        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            &query_params,
            action,
            req_body,
            common_req_headers,
        )
        .await
    }
}

fn to_header_map(headers: BTreeMap<&str, String>) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    headers.into_iter().for_each(|(key, value)| {
        header_map.insert(HeaderName::from_str(&key).unwrap(), value.parse().unwrap());
    });

    header_map
}

fn build_sored_encoded_query_string(params: &BTreeMap<&str, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v.as_str())))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod test {
    use crate::aliyun::respnose::template_respnose::TemplateResponse;

    use super::*;

    #[tokio::test]
    async fn test_send_sms() {
        // println!("key_id: {:?}", std::env::var("ALIBABA_CLOUD_ACCESS_KEY_ID"));
        // println!("key: {:?}", std::env::var("ALIBABA_CLOUD_ACCESS_KEY_SECRET"));
        let sms_service = AliyunSmsService {
            sms_client: reqwest::Client::new(),
        };
        let phone1: String = "1642312xxxx".into();
        let sign1: String = "阿里云短信测试".into();

        let result = sms_service
            .send_sms(&phone1, &sign1, "SMS_xxxxxxx", "{\"code\":\"1234\"}", None)
            .await
            .unwrap();
        println!("{:?}", result);
    }

    #[tokio::test]
    async fn test_sms_template() -> Result<(), BoxError> {
        use crate::aliyun::models::sms_template_respnose::create_respnose::CreateSmsTemplateRespnose;

        let sms_service = AliyunSmsService {
            sms_client: reqwest::Client::new(),
        };

        let _resp: TemplateResponse<CreateSmsTemplateRespnose> = sms_service
            .create_template("req_params", "", 11, None)
            .await?;

        // println!("Template name is: {}", resp.params.template_name);

        Ok(())
    }

    #[test]
    fn test_from() {
        use reqwest::header::HeaderValue;

        let val1 = HeaderValue::try_from(123).unwrap();
        let val2 = HeaderValue::try_from("123").unwrap();
        println!("val1: {:?}, val2: {:?}", val1, val2);
        assert_eq!(val1, val2)
    }
}
