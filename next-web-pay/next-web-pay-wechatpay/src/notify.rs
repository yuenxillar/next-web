use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Typed subset of WeChat Pay V2 payment notification fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentNotification {
    /// Return code.
    pub return_code: Option<String>,
    /// Return message.
    pub return_msg: Option<String>,
    /// App id.
    pub appid: Option<String>,
    /// Merchant id.
    pub mch_id: Option<String>,
    /// Nonce string.
    pub nonce_str: Option<String>,
    /// Sign.
    pub sign: Option<String>,
    /// Result code.
    pub result_code: Option<String>,
    /// User openid.
    pub openid: Option<String>,
    /// Subscription flag.
    pub is_subscribe: Option<String>,
    /// Trade type.
    pub trade_type: Option<String>,
    /// Bank type.
    pub bank_type: Option<String>,
    /// Total fee.
    pub total_fee: Option<String>,
    /// Settlement total fee.
    pub settlement_total_fee: Option<String>,
    /// Fee type.
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
}

/// Success acknowledgement XML.
pub fn success_xml() -> &'static str {
    "<xml><return_code><![CDATA[SUCCESS]]></return_code><return_msg><![CDATA[OK]]></return_msg></xml>"
}

/// Failure acknowledgement XML.
pub fn fail_xml(message: &str) -> String {
    format!(
        "<xml><return_code><![CDATA[FAIL]]></return_code><return_msg><![CDATA[{message}]]></return_msg></xml>"
    )
}

/// Converts a flat parameter map into a typed notification.
pub fn notification_from_map(
    params: &BTreeMap<String, String>,
) -> crate::WechatPayResult<PaymentNotification> {
    serde_json::from_value(serde_json::to_value(params)?).map_err(Into::into)
}
