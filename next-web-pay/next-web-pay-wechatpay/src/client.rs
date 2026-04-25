use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Identity;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::Result;
use crate::config::WechatPayConfig;
use crate::error::WechatPayError;
use crate::model::{
    ApiCommonResponse, AppPayParams, CloseOrderRequest, JsapiPayParams, MicropayRequest,
    MicropayResponse, OrderMutationResponse, OrderQueryRequest, OrderQueryResponse,
    RefundQueryRequest, RefundQueryResponse, RefundRequest, RefundResponse, RequestOptions,
    ReverseOrderRequest, SignType, UnifiedOrderRequest, UnifiedOrderResponse, WechatPayApiStatus,
};
use crate::notify::{PaymentNotification, notification_from_map};
use crate::sign::{generate_nonce_str, sign_params, verify_params};
use crate::xml::{build_xml, parse_xml};

/// WeChat Pay V2 client.
#[derive(Debug, Clone)]
pub struct WechatPayClient {
    config: WechatPayConfig,
    http_client: reqwest::Client,
}

impl WechatPayClient {
    /// Creates a client with a default HTTP client.
    pub fn new(config: WechatPayConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Creates a client with a custom HTTP client.
    pub fn with_http_client(config: WechatPayConfig, http_client: reqwest::Client) -> Self {
        Self {
            config,
            http_client,
        }
    }

    /// Returns the immutable client config.
    pub fn config(&self) -> &WechatPayConfig {
        &self.config
    }

    /// Calls the unified order API.
    pub async fn unified_order(
        &self,
        request: &UnifiedOrderRequest,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<UnifiedOrderResponse> {
        self.execute_xml("/pay/unifiedorder", request, options)
            .await
    }

    /// Calls the payment-code API.
    pub async fn micropay(
        &self,
        request: &MicropayRequest,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<MicropayResponse> {
        self.execute_xml("/pay/micropay", request, options).await
    }

    /// Queries an order.
    pub async fn order_query(
        &self,
        request: &OrderQueryRequest,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<OrderQueryResponse> {
        self.execute_xml("/pay/orderquery", request, options).await
    }

    /// Closes an unpaid order.
    pub async fn close_order(
        &self,
        request: &CloseOrderRequest,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<OrderMutationResponse> {
        self.execute_xml("/pay/closeorder", request, options).await
    }

    /// Reverses an order using the merchant certificate.
    pub async fn reverse_order(
        &self,
        request: &ReverseOrderRequest,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<OrderMutationResponse> {
        self.execute_xml_with_cert("/secapi/pay/reverse", request, options)
            .await
    }

    /// Applies for a refund using the merchant certificate.
    pub async fn refund(
        &self,
        request: &RefundRequest,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<RefundResponse> {
        self.execute_xml_with_cert("/secapi/pay/refund", request, options)
            .await
    }

    /// Queries a refund.
    pub async fn refund_query(
        &self,
        request: &RefundQueryRequest,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<RefundQueryResponse> {
        self.execute_xml("/pay/refundquery", request, options).await
    }

    /// Verifies the signature of a parsed notification map.
    pub fn verify_notification_signature(&self, params: &BTreeMap<String, String>) -> Result<bool> {
        let sign_type = sign_type_from_map(params, self.config.sign_type);
        verify_params(params, &self.config.api_key, sign_type)
    }

    /// Parses and verifies a payment notification XML payload.
    pub fn parse_payment_notification(&self, xml: &str) -> Result<PaymentNotification> {
        let params = parse_xml(xml)?;
        if !self.verify_notification_signature(&params)? {
            return Err(WechatPayError::InvalidSignature);
        }
        notification_from_map(&params)
    }

    /// Builds the frontend JSAPI pay parameters from a prepay id.
    pub fn build_jsapi_pay_params(
        &self,
        prepay_id: impl Into<String>,
        sign_type: impl Into<Option<SignType>>,
    ) -> Result<JsapiPayParams> {
        let sign_type = sign_type.into().unwrap_or(self.config.sign_type);
        let time_stamp = unix_timestamp();
        let nonce_str = generate_nonce_str();
        let package = format!("prepay_id={}", prepay_id.into());

        let mut params = BTreeMap::new();
        params.insert("appId".to_string(), self.config.app_id.clone());
        params.insert("timeStamp".to_string(), time_stamp.clone());
        params.insert("nonceStr".to_string(), nonce_str.clone());
        params.insert("package".to_string(), package.clone());
        params.insert("signType".to_string(), sign_type.as_str().to_string());
        let pay_sign = sign_params(&params, &self.config.api_key, sign_type)?;

        Ok(JsapiPayParams {
            app_id: self.config.app_id.clone(),
            time_stamp,
            nonce_str,
            package,
            sign_type: sign_type.as_str().to_string(),
            pay_sign,
        })
    }

    /// Builds the APP pay parameters from a prepay id.
    pub fn build_app_pay_params(
        &self,
        prepay_id: impl Into<String>,
        sign_type: impl Into<Option<SignType>>,
    ) -> Result<AppPayParams> {
        let sign_type = sign_type.into().unwrap_or(self.config.sign_type);
        let timestamp = unix_timestamp();
        let noncestr = generate_nonce_str();
        let prepayid = prepay_id.into();

        let mut params = BTreeMap::new();
        params.insert("appid".to_string(), self.config.app_id.clone());
        params.insert("partnerid".to_string(), self.config.mch_id.clone());
        params.insert("prepayid".to_string(), prepayid.clone());
        params.insert("package".to_string(), "Sign=WXPay".to_string());
        params.insert("noncestr".to_string(), noncestr.clone());
        params.insert("timestamp".to_string(), timestamp.clone());
        let sign = sign_params(&params, &self.config.api_key, sign_type)?;

        Ok(AppPayParams {
            appid: self.config.app_id.clone(),
            partnerid: self.config.mch_id.clone(),
            prepayid,
            package: "Sign=WXPay".to_string(),
            noncestr,
            timestamp,
            sign,
        })
    }

    async fn execute_xml<T, R>(
        &self,
        path: &str,
        request: &T,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned + WechatPayApiStatus,
    {
        let xml = self.signed_xml(request, options.into().unwrap_or_default())?;
        let url = format!("{}{}", self.config.base_url, path);
        let response = self
            .http_client
            .post(url)
            .header("content-type", "text/xml; charset=utf-8")
            .body(xml)
            .send()
            .await?;

        let body = response.text().await?;
        let map = parse_xml(&body)?;
        self.ensure_valid_signature(&map)?;
        let typed = map_to_struct::<R>(&map)?;
        self.ensure_success(&typed, &body)?;
        Ok(typed)
    }

    async fn execute_xml_with_cert<T, R>(
        &self,
        path: &str,
        request: &T,
        options: impl Into<Option<RequestOptions>>,
    ) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned + WechatPayApiStatus,
    {
        let xml = self.signed_xml(request, options.into().unwrap_or_default())?;
        let url = format!("{}{}", self.config.base_url, path);
        let client = self.build_cert_http_client()?;
        let response = client
            .post(url)
            .header("content-type", "text/xml; charset=utf-8")
            .body(xml)
            .send()
            .await?;

        let body = response.text().await?;
        let map = parse_xml(&body)?;
        self.ensure_valid_signature(&map)?;
        let typed = map_to_struct::<R>(&map)?;
        self.ensure_success(&typed, &body)?;
        Ok(typed)
    }

    fn signed_xml<T>(&self, request: &T, options: RequestOptions) -> Result<String>
    where
        T: Serialize + ?Sized,
    {
        self.validate_config()?;

        let sign_type = options.sign_type.unwrap_or(self.config.sign_type);
        let mut params = object_to_string_map(request)?;
        params.insert("appid".to_string(), self.config.app_id.clone());
        params.insert("mch_id".to_string(), self.config.mch_id.clone());
        params.insert("nonce_str".to_string(), generate_nonce_str());
        params.insert("sign_type".to_string(), sign_type.as_str().to_string());

        if let Some(notify_url) = options
            .notify_url
            .or_else(|| self.config.notify_url.clone())
        {
            params.entry("notify_url".to_string()).or_insert(notify_url);
        }

        let sign = sign_params(&params, &self.config.api_key, sign_type)?;
        params.insert("sign".to_string(), sign);
        Ok(build_xml(&params))
    }

    fn validate_config(&self) -> Result<()> {
        if self.config.app_id.trim().is_empty() {
            return Err(WechatPayError::InvalidConfig("app_id is empty".to_string()));
        }
        if self.config.mch_id.trim().is_empty() {
            return Err(WechatPayError::InvalidConfig("mch_id is empty".to_string()));
        }
        if self.config.api_key.trim().is_empty() {
            return Err(WechatPayError::InvalidConfig(
                "api_key is empty".to_string(),
            ));
        }
        Ok(())
    }

    fn build_cert_http_client(&self) -> Result<reqwest::Client> {
        let path = self.config.merchant_cert_p12_path.as_ref().ok_or_else(|| {
            WechatPayError::InvalidConfig("merchant_cert_p12_path is empty".to_string())
        })?;
        let password = self
            .config
            .merchant_cert_p12_password
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.config.mch_id.clone());
        let bytes = fs::read(path)?;
        let identity = Identity::from_pkcs12_der(&bytes, &password)
            .map_err(|error| WechatPayError::Identity(error.to_string()))?;

        reqwest::Client::builder()
            .identity(identity)
            .build()
            .map_err(Into::into)
    }

    fn ensure_valid_signature(&self, params: &BTreeMap<String, String>) -> Result<()> {
        if params.get("return_code").map(String::as_str) != Some("SUCCESS") {
            return Ok(());
        }
        if !params.contains_key("sign") {
            return Ok(());
        }

        let sign_type = sign_type_from_map(params, self.config.sign_type);
        if verify_params(params, &self.config.api_key, sign_type)? {
            Ok(())
        } else {
            Err(WechatPayError::InvalidSignature)
        }
    }

    fn ensure_success<R>(&self, response: &R, body: &str) -> Result<()>
    where
        R: WechatPayApiStatus,
    {
        let status: &ApiCommonResponse = response.api_status();
        if status.return_code.as_deref() != Some("SUCCESS") {
            return Err(WechatPayError::Transport {
                return_code: status
                    .return_code
                    .clone()
                    .unwrap_or_else(|| "FAIL".to_string()),
                return_msg: status
                    .return_msg
                    .clone()
                    .unwrap_or_else(|| "Unknown return_msg".to_string()),
                body: body.to_string(),
            });
        }

        if response.is_success() {
            return Ok(());
        }

        Err(WechatPayError::Api {
            return_msg: status.return_msg.clone(),
            err_code: status.err_code.clone(),
            err_code_des: status.err_code_des.clone(),
            body: body.to_string(),
        })
    }
}

fn object_to_string_map<T>(value: &T) -> Result<BTreeMap<String, String>>
where
    T: Serialize + ?Sized,
{
    let value = serde_json::to_value(value)?;
    let Value::Object(object) = value else {
        return Err(WechatPayError::Serde(serde_json::Error::io(
            std::io::Error::other("request must serialize to an object"),
        )));
    };

    Ok(json_object_to_map(&object))
}

fn json_object_to_map(object: &Map<String, Value>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (key, value) in object {
        if value.is_null() {
            continue;
        }

        let text = match value {
            Value::String(value) => value.clone(),
            Value::Number(value) => value.to_string(),
            Value::Bool(value) => value.to_string(),
            Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
            Value::Null => String::new(),
        };

        if !text.is_empty() {
            map.insert(key.clone(), text);
        }
    }
    map
}

fn map_to_struct<T>(map: &BTreeMap<String, String>) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(serde_json::to_value(map)?).map_err(Into::into)
}

fn sign_type_from_map(params: &BTreeMap<String, String>, fallback: SignType) -> SignType {
    match params.get("sign_type").map(String::as_str) {
        Some("HMAC-SHA256") => SignType::HmacSha256,
        Some("MD5") => SignType::Md5,
        _ => fallback,
    }
}

fn unix_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[cfg(test)]
mod tests {
    use crate::config::WechatPayConfig;
    use crate::model::{CloseOrderRequest, SignType};

    use super::{WechatPayClient, object_to_string_map};

    #[test]
    fn object_to_map_keeps_string_values() {
        let request = CloseOrderRequest {
            out_trade_no: "trade-001".to_string(),
        };

        let map = object_to_string_map(&request).expect("map");

        assert_eq!(
            map.get("out_trade_no").map(String::as_str),
            Some("trade-001")
        );
    }

    #[test]
    fn jsapi_params_can_be_built() {
        let client = WechatPayClient::new(
            WechatPayConfig::new("wx123", "1900000109", "secret").with_sign_type(SignType::Md5),
        );

        let params = client
            .build_jsapi_pay_params("wx-prepay-id", None)
            .expect("params");

        assert_eq!(params.app_id, "wx123");
        assert_eq!(params.package, "prepay_id=wx-prepay-id");
        assert!(!params.pay_sign.is_empty());
    }
}
