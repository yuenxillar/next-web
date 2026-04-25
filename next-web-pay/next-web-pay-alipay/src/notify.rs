use serde::{Deserialize, Serialize};

/// Typed subset of Alipay asynchronous notification fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentNotification {
    /// Notify time.
    pub notify_time: Option<String>,
    /// Notify type.
    pub notify_type: Option<String>,
    /// Notify id.
    pub notify_id: Option<String>,
    /// Signature type.
    pub sign_type: Option<String>,
    /// App id.
    pub app_id: Option<String>,
    /// Merchant order number.
    pub out_trade_no: Option<String>,
    /// Alipay trade number.
    pub trade_no: Option<String>,
    /// Trade status.
    pub trade_status: Option<String>,
    /// Total amount.
    pub total_amount: Option<String>,
    /// Receipt amount.
    pub receipt_amount: Option<String>,
    /// Buyer paid amount.
    pub buyer_pay_amount: Option<String>,
    /// Invoice amount.
    pub invoice_amount: Option<String>,
    /// Buyer id.
    pub buyer_id: Option<String>,
    /// Buyer logon id.
    pub buyer_logon_id: Option<String>,
    /// Seller id.
    pub seller_id: Option<String>,
    /// Payment time.
    pub gmt_payment: Option<String>,
    /// Passback params.
    pub passback_params: Option<String>,
    /// Charset.
    pub charset: Option<String>,
    /// API version.
    pub version: Option<String>,
}
