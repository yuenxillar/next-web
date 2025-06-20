use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::SystemTime};

use chrono::DateTime;
use next_web_core::{async_trait, core::service::Service, error::BoxError};
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    aliyun::respnose::sms_respnose::AliyunCloudSmsResponse,
    core::service::SmsSendService,
    signature::v3::{build_sored_encoded_query_string, hex_sha256, SignerV3},
};

#[cfg(feature = "template")]
use crate::aliyun::models::create_sms_template::*;
#[cfg(feature = "template")]
use crate::core::template_service::{TemplateResult, TemplateService};

#[derive(Clone)]
pub struct AliyunSmsService {
    sms_client: reqwest::Client,
}

impl Service for AliyunSmsService {}

const PATH: &'static str = "/";
const METHOD: &'static str = "POST";
const ENDPOINT: &'static str = "dysmsapi.aliyuncs.com";
const SMS_ACTION: &'static str = "SendSms";
const SMS_BATCH_ACTION: &'static str = "SendBatchSms";
const VERSION: &'static str = "2017-05-25";
const ALGORITHM: &'static str = "ACS3-HMAC-SHA256";

static ACCESS_KEY_ID: Lazy<Arc<String>> =
    Lazy::new(|| Arc::new(std::env::var("ALIBABA_CLOUD_ACCESS_KEY_ID").unwrap()));

static ACCESS_KEY_SECRET: Lazy<Arc<String>> =
    Lazy::new(|| Arc::new(std::env::var("ALIBABA_CLOUD_ACCESS_KEY_SECRET").unwrap()));

#[async_trait]
impl SmsSendService for AliyunSmsService {
    type Response = AliyunCloudSmsResponse;

    /// read this doc: https://next.api.aliyun.com/document/Dysmsapi/2017-05-25/SendSms
    async fn send_sms<'a>(
        &self,
        phone_numbers: &'a str,
        sign_name: &'a str,
        template_code: &'a str,
        template_param: &'a str,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> Result<Self::Response, BoxError> {
        if !self.check_validity(phone_numbers, sign_name) {
            return Err("phone_numbers or sign_names is invalid.".into());
        }

        let mut query_params: BTreeMap<&'a str, String> = BTreeMap::new();

        query_params.insert("PhoneNumbers", phone_numbers.into());
        query_params.insert("SignName", sign_name.into());
        query_params.insert("TemplateCode", template_code.into());
        query_params.insert("TemplateParam", template_param.into());

        expand_params.map(|var| query_params.extend(var));

        let req_body = "";
        let mut common_req_headers = self.common_req_headers();
        // fill
        common_req_headers.insert("x-acs-action", SMS_ACTION.into());
        common_req_headers.insert("x-acs-content-sha256", hex_sha256(req_body));

        // build SignerV3
        let signer = SignerV3::new(
            ACCESS_KEY_ID.as_str(),
            ACCESS_KEY_SECRET.as_str(),
            "service",
            "region",
            ALGORITHM,
        );
        let authorization = signer.sign(
            METHOD,
            PATH,
            &query_params,
            &common_req_headers,
            req_body.as_bytes(),
        )?;
        common_req_headers.insert("Authorization", authorization);

        // build headers
        let mut headers = HeaderMap::new();
        common_req_headers.into_iter().for_each(|(key, value)| {
            headers.insert(
                HeaderName::from_str(&key).unwrap_or(HeaderName::from_static("")),
                value.parse().unwrap_or(HeaderValue::from_static("")),
            );
        });

        let canonical_query_string = build_sored_encoded_query_string(&query_params);
        let url = format!("{}{}?{}", self.url(), PATH, canonical_query_string);

        let resp = self.sms_client.post(url).headers(headers).send().await?;

        Ok(resp.json::<Self::Response>().await?)
    }

    async fn send_batch_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_names: Vec<&'a str>,
        template_code: &'a str,
        template_params: Vec<&'a str>,
        expand_params: Option<BTreeMap<&'a str, String>>,
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

        let phone_numbers = serde_json::to_string(&phone_numbers)?;
        let sign_names = serde_json::to_string(&sign_names)?;
        let template_params = serde_json::to_string(&template_params)?;

        let mut query_params: BTreeMap<&'a str, String> = BTreeMap::new();

        query_params.insert("PhoneNumberJson", phone_numbers);
        query_params.insert("SignNameJson", sign_names);
        query_params.insert("TemplateCode", template_code.into());
        query_params.insert("TemplateParamJson", template_params);
        expand_params.map(|var| query_params.extend(var));

        let req_body = "";
        let mut common_req_headers = self.common_req_headers();
        // fill
        common_req_headers.insert("x-acs-action", SMS_BATCH_ACTION.into());
        common_req_headers.insert("x-acs-content-sha256", hex_sha256(req_body));

        // build SignerV3
        let signer = SignerV3::new(
            ACCESS_KEY_ID.as_str(),
            ACCESS_KEY_SECRET.as_str(),
            "service",
            "region",
            ALGORITHM,
        );
        let authorization = signer.sign(
            METHOD,
            PATH,
            &query_params,
            &common_req_headers,
            req_body.as_bytes(),
        )?;
        common_req_headers.insert("Authorization", authorization);

        // build headers
        let mut headers = HeaderMap::new();
        common_req_headers.into_iter().for_each(|(key, value)| {
            headers.insert(
                HeaderName::from_str(&key).unwrap_or(HeaderName::from_static("")),
                value.parse().unwrap_or(HeaderValue::from_static("")),
            );
        });

        let canonical_query_string = build_sored_encoded_query_string(&query_params);
        let url = format!("{}{}?{}", self.url(), PATH, canonical_query_string);

        let resp = self.sms_client.post(url).headers(headers).send().await?;

        Ok(resp.json::<Self::Response>().await?)
    }

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
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> TemplateResult<R>
    where
        R: DeserializeOwned,
    {
        let action: &str = "CreateSmsTemplate";

        let mut query_params = BTreeMap::new();
        query_params.insert("TemplateName", template_name.into());
        query_params.insert("TemplateContent", template_content.into());
        query_params.insert("TemplateType", template_type.to_string());

        expand_params.map(|var| query_params.extend(var));

        let resp = self.sms_client.post(self.url()).send().await?;

        Ok(resp.json::<R>().await?)
    }

    async fn delete_template<R>(&self, template_code: &str) -> TemplateResult<R>
    where
        R: DeserializeOwned,
    {
        let action: &str = "DeleteSmsTemplate";

        let url = format!("{}{}?TemplateCode={}", self.url(), PATH, urlencoding::encode(template_code));
        let resp = self.sms_client.post(url).send().await?;

        Ok(resp.json::<R>().await?)
    }

    async fn update_template<'a, R>(
        &self,
        template_name: &'a str,
        template_content: &'a str,
        template_type: i32,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> TemplateResult<R> {
        let action: &str = "UpdateSmsTemplate";

        Err("".into())
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
        let action: &str = "QuerySmsTemplateList";

        Err("".into())
    }
}

#[cfg(test)]
mod test {
    use crate::aliyun::respnose::template_respnose::TemplateResponse;

    use super::*;

    #[tokio::test]
    async fn test_send_sms() {
        let sms_service = AliyunSmsService {
            sms_client: reqwest::Client::new(),
        };
        let phone1: String = "13542313xxxx".into();
        let phone2: String = "13542366613xxxx".into();

        let sign1: String = "sadwadawdaw".into();
        let sign2: String = "zasdwadawd".into();

        let param1: String = "sadwadawdaw".into();
        let param2: String = "zasdwadawd".into();

        let result = sms_service
            .send_batch_sms(
                vec![&phone1, &phone2],
                vec![&sign1, &sign2],
                "kasdllwwl-w1s3a35wd1",
                vec![&param1, &param2],
                None,
            )
            .await
            .unwrap();
        println!("{:?}", uuid::Uuid::new_v4().to_string());
    }

    #[tokio::test]
    async fn test_sms_template() -> Result<(), BoxError> {
        let sms_service = AliyunSmsService {
            sms_client: reqwest::Client::new(),
        };

        let req_params = CreateSmsTemplateRequest {
            template_name: Default::default(),
            template_content: Default::default(),
            remark: Default::default(),
            template_type: Default::default(),
            related_sign_name: Default::default(),
            template_rule: Default::default(),
            more_data: Default::default(),
            apply_scene_content: Default::default(),
            intl_type: Default::default(),
        };
        let resp: TemplateResponse<CreateSmsTemplateRespnose> =
            sms_service.create_template("req_params", "" , 11, None).await?;

        println!("Template name is: {}", resp.params.template_name);

        Ok(())
    }
}
