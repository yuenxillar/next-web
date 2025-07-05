use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use next_web_core::{async_trait, core::service::Service, error::BoxError};
use once_cell::sync::Lazy;
use reqwest::{
    header::{HeaderMap, HeaderName},
    Method,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    core::signer::SignerV3,
    tencent::{respnose::sms_respnose::TencentCloudSmsResponse, signer::TencentCloudSigner},
};

use crate::core::service::sms_service::SmsService;

#[cfg(feature = "template")]
use crate::core::service::template_service::TemplateService;

#[cfg(feature = "sign")]
use crate::core::service::sign_service::SignService;

const SMS_ENDPOINT: &'static str = "sms.tencentcloudapi.com";
const SMS_VERSION: &'static str = "2021-01-11";
const LANGUAGE: &'static str = "zh-CN";
const PATH: &'static str = "/";
const ALGORITHM: &'static str = "TC3-HMAC-SHA256";
const JSON_CONTENT_TYPE: &'static str = "application/json; charset=utf-8";
const QUERY_CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
const REGION: &'static str = "ap-guangzhou";

static TENCENTCLOUD_SECRET_ID: Lazy<Arc<String>> =
    Lazy::new(|| Arc::new(std::env::var("TENCENTCLOUD_SECRET_ID").unwrap()));

static TENCENTCLOUD_SECRET_KEY: Lazy<Arc<String>> =
    Lazy::new(|| Arc::new(std::env::var("TENCENTCLOUD_SECRET_KEY").unwrap()));

#[derive(Clone)]
pub struct TencentCloudSmsService {
    sms_client: reqwest::Client,
}

impl TencentCloudSmsService {
    pub fn new() -> Self {
        Self {
            sms_client: reqwest::Client::new(),
        }
    }

    async fn call_api<R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        params: Option<BTreeMap<&str, String>>,
        mut common_req_headers: BTreeMap<&str, String>,
        body: impl Into<String>,
        secret_key: &str,
        secret_key_id: &str,
        algorithm: &str,
        action: &str,
    ) -> Result<R, BoxError> {
        let url = self.url();
        let body: String = body.into();
        if method.to_uppercase().eq(Method::POST.as_str()) {
            common_req_headers.insert("Content-Type", JSON_CONTENT_TYPE.into());
        } else if method.to_uppercase().eq(Method::GET.as_str()) {
            common_req_headers.insert("Content-Type", QUERY_CONTENT_TYPE.into());
        }

        common_req_headers.insert("X-TC-Action", action.into());

        let signer = TencentCloudSigner::new("sms");
        let authorization = signer.sign(
            method,
            path,
            params.as_ref(),
            &common_req_headers,
            &body,
            secret_key,
            secret_key_id,
            algorithm,
        )?;
        common_req_headers.insert("Authorization", authorization);

        let headers = to_header_map(common_req_headers);

        let resp = if method.to_uppercase().eq(Method::POST.as_str()) {
            self.sms_client
                .post(url)
                .body(body)
                .headers(headers)
                .send()
                .await?
        } else {
            self.sms_client
                .get(url)
                .query(&params.unwrap_or_default())
                .headers(headers)
                .send()
                .await?
        };

        Ok(resp.json::<R>().await?)
    }
}

impl Service for TencentCloudSmsService {}

#[async_trait]
impl SmsService for TencentCloudSmsService {
    type Response = TencentCloudSmsResponse;

    /// read this doc: https://cloud.tencent.com/document/api/382
    async fn send_sms<'a>(
        &self,
        phone_number: &'a str,
        sign_name: &'a str,
        template_code: &'a str,
        template_param: &'a str,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<Self::Response, BoxError> {
        let phone_numbers: Vec<&str> =
            if let Ok(phone_numbers) = serde_json::from_str(&phone_number) {
                phone_numbers
            } else {
                vec![phone_number]
            };
        if phone_numbers.len() < 1 {
            return Err("phone_number is empty!".into());
        }

        if !phone_numbers
            .iter()
            .all(|phone| self.check_validity(phone, sign_name))
        {
            return Err("phone_number invalid! please check each number.".into());
        }

        if !expand_params
            .as_ref()
            .map(|var| {
                let app_id = var.get("SmsSdkAppId");
                let is_valid = is_number(app_id.map(|v| v.as_str().unwrap_or("a")).unwrap_or("b"));
                is_valid
            })
            .unwrap_or_default()
        {
            return Err("expand_params missing SmsSdkAppId!".into());
        }

        if template_code.is_empty() {
            return Err("template_code is empty!".into());
        }

        let template_params: Vec<&str> =
            if let Ok(template_params) = serde_json::from_str(&template_param) {
                template_params
            } else {
                vec![template_param]
            };

        let action = "SendSms";
        let mut params: BTreeMap<&str, Value> = BTreeMap::new();

        params.insert("PhoneNumberSet", phone_numbers.into());
        params.insert("SignName", sign_name.into());

        params.insert("TemplateId", template_code.into());
        params.insert("TemplateParamSet", template_params.into());

        expand_params.map(|var| params.extend(var));

        let body = serde_json::to_string(&params)?;

        let common_req_headers = self.common_req_headers();
        self.call_api::<Self::Response>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
        )
        .await
    }

    async fn send_batch_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_name: Vec<&'a str>,
        template_code: &'a str,
        template_params: Vec<&'a str>,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<Self::Response, BoxError> {
        if sign_name.len() == 0 || sign_name.len() > 1 {
            return Err("sign_name only supports individual.".into());
        }
        let phone_numbers = serde_json::to_string(&phone_numbers)?;
        let template_params = serde_json::to_string(&template_params)?;
        let sign_name = sign_name[0];

        self.send_sms(
            &phone_numbers,
            sign_name,
            template_code,
            &template_params,
            expand_params,
        )
        .await
    }

    fn check_validity<'a>(&self, phone_number: &'a str, sign_name: &'a str) -> bool {
        if phone_number.is_empty() || phone_number.len() > 20 {
            return false;
        }

        if sign_name.is_empty() {
            return false;
        }

        true
    }

    fn url(&self) -> &str {
        "https://sms.tencentcloudapi.com"
    }

    fn common_req_headers(&self) -> BTreeMap<&str, String> {
        let mut headers = BTreeMap::new();

        headers.insert("Host", SMS_ENDPOINT.into());

        let unix_timestamp = chrono::Utc::now().timestamp();
        headers.insert("X-TC-Timestamp", unix_timestamp.to_string());
        headers.insert("X-TC-Version", SMS_VERSION.into());
        headers.insert("X-TC-Language", LANGUAGE.into());
        headers.insert("X-TC-Region", REGION.into());

        headers
    }
}

#[cfg(feature = "template")]
#[async_trait]
impl TemplateService for TencentCloudSmsService {
    async fn create_template<'a, R>(
        &self,
        template_name: &'a str,
        template_content: &'a str,
        template_type: i32,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned,
    {
        if template_type <= 0 || template_type > 3 {
            return Err("template_type invalid!".into());
        }

        if template_name.is_empty() || template_content.is_empty() {
            return Err("template_content or template_name is empty!".into());
        }

        let error_str = expand_params
            .as_ref()
            .map(|params| {
                let international = params.get("International");
                let remark = params.get("Remark");
                let is_valid = international
                    .map(|s| s.as_i64().unwrap_or(-1))
                    .map(|s| s >= 0 && s < 2)
                    .unwrap_or_default();
                if remark.is_none() {
                    return "expand_params missing [Remark] parameter.";
                }
                if !is_valid {
                    return "expand_params [International] parameter invalid.";
                }
                Default::default()
            })
            .unwrap_or("expand_params missing [International] and [Remark] parameter.");

        if !error_str.is_empty() {
            return Err(error_str.into());
        }

        let action = "AddSmsTemplate";
        let mut params: BTreeMap<&str, Value> = BTreeMap::new();
        params.insert("TemplateName", template_name.into());
        params.insert("TemplateContent", template_content.into());
        params.insert("SmsType", template_type.into());

        expand_params.map(|var| params.extend(var));
        let common_req_headers = self.common_req_headers();

        let body = serde_json::to_string(&params)?;
        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            &body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
        )
        .await
    }

    async fn delete_template<R>(&self, template_code: &str) -> Result<R, BoxError>
    where
        R: DeserializeOwned,
    {
        if template_code.is_empty() {
            return Err("template_code is empty!".into());
        }

        let action = "DeleteSmsTemplate";
        let mut params: BTreeMap<&str, Value> = BTreeMap::new();
        params.insert(
            "TemplateId",
            Value::Number(template_code.parse::<u64>().unwrap_or_default().into()),
        );

        let common_req_headers = self.common_req_headers();

        let body = serde_json::to_string(&params)?;
        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            &body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
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
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned,
    {
        if template_type <= 0 || template_type > 3 {
            return Err("template_type invalid!".into());
        }

        if template_name.is_empty() || template_content.is_empty() {
            return Err("template_content or template_name is empty!".into());
        }

        let error_str = expand_params
            .as_ref()
            .map(|params| {
                let international = params.get("International");
                let remark = params.get("Remark");
                let is_valid = international
                    .map(|s| s.as_i64().unwrap_or(-1))
                    .map(|s| s >= 0 && s < 2)
                    .unwrap_or_default();
                if remark.is_none() {
                    return "expand_params missing [Remark] parameter.";
                }
                if !is_valid {
                    return "expand_params [International] parameter invalid.";
                }
                Default::default()
            })
            .unwrap_or("expand_params missing [International] and [Remark] parameter.");

        if !error_str.is_empty() {
            return Err(error_str.into());
        }

        let action = "ModifySmsTemplate";
        let mut params: BTreeMap<&str, Value> = BTreeMap::new();
        params.insert(
            "TemplateId",
            Value::Number(template_code.parse::<u64>().unwrap_or_default().into()),
        );
        params.insert("TemplateName", template_name.into());
        params.insert("TemplateContent", template_content.into());
        params.insert("SmsType", template_type.into());

        expand_params.map(|var| params.extend(var));

        let common_req_headers = self.common_req_headers();

        let body = serde_json::to_string(&params)?;
        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            &body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
        )
        .await
    }

    async fn query_template<R>(
        &self,
        international: i32,
        index: u16,
        size: u16,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned,
    {
        if international < 0 || international > 1 {
            return Err("international invalid!".into());
        }

        if size > 100 {
            return Err("size invalid, The maximum value is 100.".into());
        }

        let action = "DescribeSmsTemplateList";
        let mut params: BTreeMap<&str, Value> = BTreeMap::new();
        params.insert("International", international.into());
        // 默认全部查询 暂定
        params.insert("TemplateIdSet", Value::Array(vec![]));
        params.insert("Limit", size.into());
        params.insert("Offset", index.into());

        let common_req_headers = self.common_req_headers();

        let body = serde_json::to_string(&params)?;
        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            &body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
        )
        .await
    }
}

#[cfg(feature = "sign")]
#[async_trait]
impl SignService for TencentCloudSmsService {
    async fn create_sign<'a, R>(
        &self,
        sign_name: &'a str,
        sign_type: i32,
        sign_purpose: i32,
        qualification_id: u64,
        remark: Option<&'a str>,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned,
    {
        if !expand_params
            .as_ref()
            .map(|var| var.get("DocumentType").is_some())
            .unwrap_or_default()
        {
            return Err("expand_params missing [DocumentType] parameter.".into());
        }

        let mut international: bool = false;
        if !expand_params
            .as_ref()
            .map(|var| {
                var.get("International")
                    .map(|v| match v.as_i64().unwrap_or(-1) {
                        0 => true,
                        1 => {
                            international = true;
                            true
                        }
                        _ => false,
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default()
        {
            return Err("expand_params missing [International: i64] parameter.".into());
        }

        if !expand_params
            .as_ref()
            .map(|var| var.get("ProofImage").is_some())
            .unwrap_or_default()
        {
            return Err("expand_params missing [ProofImage] parameter.".into());
        }

        if sign_purpose == 1 {
            if !expand_params
                .as_ref()
                .map(|var| var.get("CommissionImage").is_some())
                .unwrap_or_default()
            {
                return Err("expand_params missing [CommissionImage: String] parameter.".into());
            }
        }

        let action = "AddSmsSign";
        let mut params: BTreeMap<&str, Value> = BTreeMap::new();
        params.insert("SignName", sign_name.into());
        params.insert("SignType", sign_type.into());
        params.insert("SignPurpose", sign_purpose.into());
        if international {
            params.insert("QualificationId", qualification_id.into());
        }

        remark.map(|s| params.insert("Remark", s.into()));
        expand_params.map(|var| params.extend(var));

        let common_req_headers = self.common_req_headers();

        let body = serde_json::to_string(&params)?;
        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            &body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
        )
        .await
    }

    async fn delete_sign<R>(&self, sign_id: &str) -> Result<R, BoxError>
    where
        R: DeserializeOwned,
    {
        if sign_id.is_empty() {
            return Err("sign_id is empty!".into());
        }
        let action = "DeleteSmsSign";
        let mut params: BTreeMap<&str, Value> = BTreeMap::new();
        params.insert(
            "SignId",
            Value::Number(sign_id.parse::<u64>()?.into()),
        );

        let common_req_headers = self.common_req_headers();

        let body = serde_json::to_string(&params)?;
        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            &body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
        )
        .await
    }

    async fn update_sign<'a, R>(
        &self,
        sign_name: &'a str,
        sign_type: i32,
        sign_purpose: i32,
        qualification_id: u64,
        remark: Option<&'a str>,
        expand_params: Option<BTreeMap<&'a str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned,
    {
        if !expand_params
            .as_ref()
            .map(|var| var.get("DocumentType").is_some())
            .unwrap_or_default()
        {
            return Err("expand_params missing [DocumentType: u64] parameter.".into());
        }

        let mut international: bool = false;
        if !expand_params
            .as_ref()
            .map(|var| {
                var.get("International")
                    .map(|v| match v.as_i64().unwrap_or(-1) {
                        0 => true,
                        1 => {
                            international = true;
                            true
                        }
                        _ => false,
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default()
        {
            return Err("expand_params missing [International: i64] parameter.".into());
        }

        if !expand_params
            .as_ref()
            .map(|var| var.get("ProofImage").is_some())
            .unwrap_or_default()
        {
            return Err("expand_params missing [ProofImage: String] parameter.".into());
        }

        if sign_purpose == 1 {
            if !expand_params
                .as_ref()
                .map(|var| var.get("CommissionImage").is_some())
                .unwrap_or_default()
            {
                return Err("expand_params missing [CommissionImage: String] parameter.".into());
            }
        }

        let action = "ModifySmsSign";
        let mut params: BTreeMap<&str, Value> = BTreeMap::new();
        params.insert("SignName", sign_name.into());
        params.insert("SignType", sign_type.into());
        params.insert("SignPurpose", sign_purpose.into());
        if international {
            params.insert("QualificationId", qualification_id.into());
        }

        remark.map(|s| params.insert("Remark", s.into()));
        expand_params.map(|var| params.extend(var));

        let common_req_headers = self.common_req_headers();

        let body = serde_json::to_string(&params)?;
        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            &body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
        )
        .await
    }

    async fn query_sign<R>(
        &self,
        sign_id: &str,
        expand_params: Option<BTreeMap<&str, Value>>,
    ) -> Result<R, BoxError>
    where
        R: DeserializeOwned,
    {
        if sign_id.is_empty() {
            return Err("sign_id is empty!".into());
        }

        if !expand_params
            .as_ref()
            .map(|var| var.get("International").is_some())
            .unwrap_or_default()
        {
            return Err("expand_params missing [International] parameter.".into());
        }

        let sign_id_set: Vec<Value> = if sign_id.contains(",") {
            sign_id
                .split(",")
                .filter_map(|v| v.parse::<u64>().ok())
                .map(|v| Value::Number(v.into()))
                .collect()
        } else {
            if let Ok(num) = sign_id.parse::<u64>() {
                vec![Value::Number(num.into())]
            }else { vec![] }
        };

        let mut params: BTreeMap<&str, Value> = BTreeMap::new();
        params.insert("SignIdSet", Value::Array(sign_id_set));

        expand_params.map(|var| params.extend(var));

        let action = "DescribeSmsSignList";
        let common_req_headers = self.common_req_headers();

        let body = serde_json::to_string(&params)?;
        self.call_api::<R>(
            Method::POST.as_str(),
            PATH,
            None,
            common_req_headers,
            &body,
            TENCENTCLOUD_SECRET_KEY.as_str(),
            TENCENTCLOUD_SECRET_ID.as_str(),
            ALGORITHM,
            action,
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

fn is_number(s: impl AsRef<str>) -> bool {
    let s = s.as_ref();
    s.chars().all(|c| c.is_numeric())
}

#[cfg(test)]
mod tencent_sms_test {

    use std::{collections::BTreeMap, time::Instant};

    use serde_json::Value;

    use crate::{
        core::service::{
            sign_service::SignService, sms_service::SmsService, template_service::TemplateService,
        },
        tencent::{
            respnose::{
                sign_respnose::TencentCloudSignResponse,
                template_respnose::TencentCloudTemplateResponse,
            },
            service::tencent_cloud_sms_service::TencentCloudSmsService,
        },
    };

    #[tokio::test]
    async fn test_send_sms() {
        let start = Instant::now();
        let sms_service = TencentCloudSmsService::new();
        let mut expand_params = BTreeMap::new();
        expand_params.insert("SmsSdkAppId", "1234567890".into());
        let resp = sms_service
            .send_sms(
                "178116xxxxx",
                "sign_name",
                "template_code",
                "template_param",
                Some(expand_params),
            )
            .await
            .unwrap();
        println!("resp: {:?}", resp);
        println!("time elapsed: {:?}", start.elapsed());

        let mut expand_params: BTreeMap<&str, Value> = BTreeMap::new();
        expand_params.insert("International", 0.into());
        expand_params.insert("Remark", "test".into());

        let resp1: TencentCloudTemplateResponse = sms_service
            .create_template("template_name", "template_content", 1, Some(expand_params))
            .await
            .unwrap();
        println!("resp1: {:?}", resp1.response);
    }

    #[tokio::test]
    async fn test_query_sign() {
        let sms_service = TencentCloudSmsService::new();
        let mut expand_params = BTreeMap::new();
        expand_params.insert("International", 0.into());
        let resp: TencentCloudSignResponse = sms_service
            .query_sign("1234567890", Some(expand_params))
            .await
            .unwrap();
        println!("resp: {:?}", resp);

        let mut expand_params: BTreeMap<&str, Value> = BTreeMap::new();
        expand_params.insert("International", 0.into());
        expand_params.insert("Remark", "test".into());

        let resp1: TencentCloudTemplateResponse = sms_service
            .create_template("template_name", "template_content", 1, Some(expand_params))
            .await
            .unwrap();
        println!("resp1: {:?}", resp1.response);
    }
}
