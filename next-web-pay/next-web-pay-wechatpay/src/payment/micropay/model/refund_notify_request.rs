use serde::{Deserialize, Serialize};

use crate::payment::model::common::Currency;

// ==================== 通知请求（微信发送到商户） ====================

/// 退款结果通知请求（加密）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatPayRefundNotifyRequest {
    /// 通知ID
    pub id: String,

    /// 通知创建时间（RFC3339格式）
    pub create_time: String,

    /// 通知类型
    pub event_type: RefundEventType,

    /// 通知数据类型
    pub resource_type: String,

    /// 通知简要说明
    pub summary: String,

    /// 通知数据（加密）
    pub resource: EncryptedResource,
}

/// 加密资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedResource {
    /// 加密算法类型
    pub algorithm: String,

    /// 加密前的对象类型
    pub original_type: String,

    /// Base64编码的密文
    pub ciphertext: String,

    /// 附加数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub associated_data: Option<String>,

    /// 随机串
    pub nonce: String,
}

/// 解密后的退款结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResource {
    /// 商户号
    pub mchid: String,

    /// 机构商户号
    pub sp_mchid: String,

    /// 子商户号
    pub sub_mchid: String,

    /// 商户订单号
    pub out_trade_no: String,

    /// 微信支付订单号
    pub transaction_id: String,

    /// 商户退款单号
    pub out_refund_no: String,

    /// 微信退款单号
    pub refund_id: String,

    /// 退款状态
    pub refund_status: RefundNotityStatus,

    /// 退款成功时间（RFC3339格式）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_time: Option<String>,

    /// 退款入账账户
    pub recv_account: String,

    /// 退款资金来源
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fund_source: Option<FundSource>,

    /// 金额信息
    pub amount: RefundAmount,
}

/// 退款金额信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundAmount {
    /// 订单总金额
    pub total: u32,

    /// 订单标价币种
    pub currency: Currency,

    /// 退款金额
    pub refund: u32,

    /// 用户支付金额
    pub payer_total: u32,

    /// 用户退款金额
    pub payer_refund: u32,

    /// 用户支付币种
    pub payer_currency: Currency,

    /// 汇率信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<ExchangeRate>,
}

/// 汇率信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    /// 汇率类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ExchangeRateType>,

    /// 汇率值（乘以10的8次方）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<u32>,
}

// ==================== 通知应答（商户返回给微信） ====================

/// 通知应答
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyResponse {
    /// 返回状态码
    pub code: ResponseCode,

    /// 返回信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 返回状态码
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseCode {
    /// 成功
    #[serde(rename = "SUCCESS")]
    Success,

    /// 失败
    #[serde(rename = "FAIL")]
    Fail,
}

/// 通知类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefundEventType {
    /// 退款成功通知
    #[serde(rename = "REFUND.SUCCESS")]
    RefundSuccess,

    /// 退款关闭通知
    #[serde(rename = "REFUND.CLOSED")]
    RefundClosed,
}

/// 退款状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefundNotityStatus {
    /// 退款成功
    #[serde(rename = "SUCCESS")]
    Success,

    /// 退款关闭
    #[serde(rename = "CLOSED")]
    Closed,

    /// 退款异常
    #[serde(rename = "ABNORMAL")]
    Abnormal,
}

/// 退款资金来源
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundSource {
    /// 未结算资金退款
    #[serde(rename = "REFUND_SOURCE_UNSETTLED_FUNDS")]
    UnsettledFunds,

    /// 可用余额退款
    #[serde(rename = "REFUND_SOURCE_RECHARGE_FUNDS")]
    RechargeFunds,
}

/// 汇率类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExchangeRateType {
    /// 标价币种和支付币种的汇率
    #[serde(rename = "USERPAYMENT_RATE")]
    UserpaymentRate,

    /// 标价币种和结算币种的汇率
    #[serde(rename = "SETTLEMENT_RATE")]
    SettlementRate,
}
