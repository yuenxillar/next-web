use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// WeChat Pay sign type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignType {
    /// MD5 signing.
    #[default]
    #[serde(rename = "MD5")]
    Md5,
    /// HMAC-SHA256 signing.
    #[serde(rename = "HMAC-SHA256")]
    HmacSha256,
}

impl SignType {
    /// Returns the protocol string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::HmacSha256 => "HMAC-SHA256",
        }
    }
}

/// Unified order trade type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeType {
    /// Official account or mini-program payment.
    #[default]
    #[serde(rename = "JSAPI")]
    Jsapi,
    /// Native QR code payment.
    #[serde(rename = "NATIVE")]
    Native,
    /// App payment.
    #[serde(rename = "APP")]
    App,
    /// H5 payment.
    #[serde(rename = "MWEB")]
    Mweb,
}

/// Per-request gateway overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestOptions {
    /// Overrides the request notify URL.
    pub notify_url: Option<String>,
    /// Overrides the sign type.
    pub sign_type: Option<SignType>,
}

/// Common V2 response fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiCommonResponse {
    /// Return code.
    pub return_code: Option<String>,
    /// Return message.
    pub return_msg: Option<String>,
    /// Result code.
    pub result_code: Option<String>,
    /// App id.
    pub appid: Option<String>,
    /// Merchant id.
    pub mch_id: Option<String>,
    /// Device info.
    pub device_info: Option<String>,
    /// Nonce string.
    pub nonce_str: Option<String>,
    /// Sign type.
    pub sign_type: Option<String>,
    /// Response signature.
    pub sign: Option<String>,
    /// Error code.
    pub err_code: Option<String>,
    /// Error description.
    pub err_code_des: Option<String>,
}

/// Common status access for typed responses.
pub trait WechatPayApiStatus {
    /// Returns the shared response status block.
    fn api_status(&self) -> &ApiCommonResponse;

    /// Returns whether the API call succeeded.
    fn is_success(&self) -> bool {
        self.api_status().return_code.as_deref() == Some("SUCCESS")
            && self.api_status().result_code.as_deref() == Some("SUCCESS")
    }
}

/// Unified order request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedOrderRequest {
    /// Order description.
    pub body: String,
    /// Merchant order number.
    pub out_trade_no: String,
    /// Total amount in fen.
    pub total_fee: String,
    /// Client IP.
    pub spbill_create_ip: String,
    /// Trade type.
    pub trade_type: TradeType,
    /// Device info.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_info: Option<String>,
    /// Goods detail JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Attach data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<String>,
    /// Currency type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_type: Option<String>,
    /// Order start time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_start: Option<String>,
    /// Order expiry time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_expire: Option<String>,
    /// Goods tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_tag: Option<String>,
    /// Product id required by NATIVE.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    /// Limit pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_pay: Option<String>,
    /// User openid required by JSAPI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openid: Option<String>,
    /// Electronic invoice flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
    /// Scene info JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_info: Option<String>,
    /// Profit sharing flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profit_sharing: Option<String>,
    /// Additional fields.
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

impl UnifiedOrderRequest {
    /// Creates a minimal unified order request.
    pub fn new(
        body: impl Into<String>,
        out_trade_no: impl Into<String>,
        total_fee: impl Into<String>,
        spbill_create_ip: impl Into<String>,
        trade_type: TradeType,
    ) -> Self {
        Self {
            body: body.into(),
            out_trade_no: out_trade_no.into(),
            total_fee: total_fee.into(),
            spbill_create_ip: spbill_create_ip.into(),
            trade_type,
            ..Self::default()
        }
    }
}

/// Payment code request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MicropayRequest {
    /// Order description.
    pub body: String,
    /// Merchant order number.
    pub out_trade_no: String,
    /// Total amount in fen.
    pub total_fee: String,
    /// Client IP.
    pub spbill_create_ip: String,
    /// Buyer payment code.
    pub auth_code: String,
    /// Device info.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_info: Option<String>,
    /// Goods detail JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Attach data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<String>,
    /// Currency type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_type: Option<String>,
    /// Goods tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_tag: Option<String>,
    /// Limit pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_pay: Option<String>,
    /// Electronic invoice flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
    /// Scene info JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_info: Option<String>,
    /// Profit sharing flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profit_sharing: Option<String>,
    /// Additional fields.
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

impl MicropayRequest {
    /// Creates a minimal payment code request.
    pub fn new(
        body: impl Into<String>,
        out_trade_no: impl Into<String>,
        total_fee: impl Into<String>,
        spbill_create_ip: impl Into<String>,
        auth_code: impl Into<String>,
    ) -> Self {
        Self {
            body: body.into(),
            out_trade_no: out_trade_no.into(),
            total_fee: total_fee.into(),
            spbill_create_ip: spbill_create_ip.into(),
            auth_code: auth_code.into(),
            ..Self::default()
        }
    }
}

/// Order query request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderQueryRequest {
    /// WeChat transaction id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    /// Merchant order number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
}

/// Close order request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CloseOrderRequest {
    /// Merchant order number.
    pub out_trade_no: String,
}

/// Reverse order request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReverseOrderRequest {
    /// WeChat transaction id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    /// Merchant order number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
}

/// Refund request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefundRequest {
    /// WeChat transaction id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    /// Merchant order number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    /// Merchant refund number.
    pub out_refund_no: String,
    /// Order total amount in fen.
    pub total_fee: String,
    /// Refund amount in fen.
    pub refund_fee: String,
    /// Currency type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_fee_type: Option<String>,
    /// Refund notify URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_url: Option<String>,
    /// Refund account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_account: Option<String>,
    /// Refund description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_desc: Option<String>,
    /// Additional fields.
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

/// Refund query request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefundQueryRequest {
    /// WeChat transaction id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    /// Merchant order number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    /// Merchant refund number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_refund_no: Option<String>,
    /// WeChat refund id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_id: Option<String>,
}

/// Unified order response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedOrderResponse {
    /// Common response fields.
    #[serde(flatten)]
    pub api: ApiCommonResponse,
    /// Trade type.
    pub trade_type: Option<String>,
    /// Prepay id.
    pub prepay_id: Option<String>,
    /// Native QR url.
    pub code_url: Option<String>,
    /// H5 payment URL.
    pub mweb_url: Option<String>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl WechatPayApiStatus for UnifiedOrderResponse {
    fn api_status(&self) -> &ApiCommonResponse {
        &self.api
    }
}

/// Payment code response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MicropayResponse {
    /// Common response fields.
    #[serde(flatten)]
    pub api: ApiCommonResponse,
    /// User openid.
    pub openid: Option<String>,
    /// Subscription flag.
    pub is_subscribe: Option<String>,
    /// Trade type.
    pub trade_type: Option<String>,
    /// Bank type.
    pub bank_type: Option<String>,
    /// Total fee in fen.
    pub total_fee: Option<String>,
    /// Currency type.
    pub fee_type: Option<String>,
    /// Cash fee.
    pub cash_fee: Option<String>,
    /// Cash fee type.
    pub cash_fee_type: Option<String>,
    /// WeChat transaction id.
    pub transaction_id: Option<String>,
    /// Merchant order number.
    pub out_trade_no: Option<String>,
    /// Attach data.
    pub attach: Option<String>,
    /// End time.
    pub time_end: Option<String>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl MicropayResponse {
    /// Returns whether the order should be queried again because the user is still paying.
    pub fn should_retry_query(&self) -> bool {
        self.api.err_code.as_deref() == Some("USERPAYING")
    }
}

impl WechatPayApiStatus for MicropayResponse {
    fn api_status(&self) -> &ApiCommonResponse {
        &self.api
    }
}

/// Order query response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderQueryResponse {
    /// Common response fields.
    #[serde(flatten)]
    pub api: ApiCommonResponse,
    /// User openid.
    pub openid: Option<String>,
    /// Subscription flag.
    pub is_subscribe: Option<String>,
    /// Trade type.
    pub trade_type: Option<String>,
    /// Trade state.
    pub trade_state: Option<String>,
    /// Trade state description.
    pub trade_state_desc: Option<String>,
    /// Bank type.
    pub bank_type: Option<String>,
    /// Total fee.
    pub total_fee: Option<String>,
    /// Settlement total fee.
    pub settlement_total_fee: Option<String>,
    /// Currency type.
    pub fee_type: Option<String>,
    /// Cash fee.
    pub cash_fee: Option<String>,
    /// WeChat transaction id.
    pub transaction_id: Option<String>,
    /// Merchant order number.
    pub out_trade_no: Option<String>,
    /// Attach data.
    pub attach: Option<String>,
    /// End time.
    pub time_end: Option<String>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl WechatPayApiStatus for OrderQueryResponse {
    fn api_status(&self) -> &ApiCommonResponse {
        &self.api
    }
}

/// Response used by close and reverse APIs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderMutationResponse {
    /// Common response fields.
    #[serde(flatten)]
    pub api: ApiCommonResponse,
    /// Recall flag for reverse API.
    pub recall: Option<String>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl WechatPayApiStatus for OrderMutationResponse {
    fn api_status(&self) -> &ApiCommonResponse {
        &self.api
    }
}

/// Refund response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefundResponse {
    /// Common response fields.
    #[serde(flatten)]
    pub api: ApiCommonResponse,
    /// WeChat transaction id.
    pub transaction_id: Option<String>,
    /// Merchant order number.
    pub out_trade_no: Option<String>,
    /// Merchant refund number.
    pub out_refund_no: Option<String>,
    /// WeChat refund id.
    pub refund_id: Option<String>,
    /// Refund fee.
    pub refund_fee: Option<String>,
    /// Total fee.
    pub total_fee: Option<String>,
    /// Cash fee.
    pub cash_fee: Option<String>,
    /// Settlement refund fee.
    pub settlement_refund_fee: Option<String>,
    /// Cash refund fee.
    pub cash_refund_fee: Option<String>,
    /// Currency type.
    pub fee_type: Option<String>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl WechatPayApiStatus for RefundResponse {
    fn api_status(&self) -> &ApiCommonResponse {
        &self.api
    }
}

/// Refund query response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefundQueryResponse {
    /// Common response fields.
    #[serde(flatten)]
    pub api: ApiCommonResponse,
    /// WeChat transaction id.
    pub transaction_id: Option<String>,
    /// Merchant order number.
    pub out_trade_no: Option<String>,
    /// Total fee.
    pub total_fee: Option<String>,
    /// Settlement total fee.
    pub settlement_total_fee: Option<String>,
    /// Currency type.
    pub fee_type: Option<String>,
    /// Cash fee.
    pub cash_fee: Option<String>,
    /// Refund count.
    pub refund_count: Option<String>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl WechatPayApiStatus for RefundQueryResponse {
    fn api_status(&self) -> &ApiCommonResponse {
        &self.api
    }
}

/// JSAPI payment parameters returned to frontend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsapiPayParams {
    /// App id.
    pub app_id: String,
    /// Timestamp in seconds.
    pub time_stamp: String,
    /// Nonce string.
    pub nonce_str: String,
    /// Package field such as `prepay_id=...`.
    pub package: String,
    /// Sign type.
    pub sign_type: String,
    /// paySign used by frontend SDK.
    pub pay_sign: String,
}

/// APP payment parameters returned to mobile app.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppPayParams {
    /// App id.
    pub appid: String,
    /// Merchant id.
    pub partnerid: String,
    /// Prepay id.
    pub prepayid: String,
    /// Package value.
    pub package: String,
    /// Nonce string.
    pub noncestr: String,
    /// Timestamp in seconds.
    pub timestamp: String,
    /// Signature.
    pub sign: String,
}
