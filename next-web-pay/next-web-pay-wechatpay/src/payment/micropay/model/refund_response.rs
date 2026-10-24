use serde::Deserialize;

use crate::payment::{
    micropay::model::query_order_response::PromotionScope, model::common::Currency,
};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WechatPayRefundResponse {
    /// 微信支付订单号
    pub id: String,

    /// 商户退款单号
    pub out_refund_no: String,

    /// 退款创建时间
    pub create_time: String,

    /// 退款金额
    pub amount: RefundRespAmount,

    /// 优惠退款详情
    pub detail: Option<Vec<DiscountedRefund>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RefundRespAmount {
    /// 退款金额
    pub refund: u32,

    /// 退款币种
    pub currency: Currency,

    /// 用户退款金额
    pub payer_refund: u32,

    /// 支付币种
    pub payer_currency: Currency,

    /// 结算币种退款金额
    pub settlement_refund: u32,

    /// 结算币种
    pub settlement_currency: Currency,

    /// 汇率
    pub exchange_rate: RefundRespExchangeRate,

    /// 优惠退款信息
    pub from: Option<Vec<RefundSourceRespInfo>>,
}

/// Exchange rate info.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RefundRespExchangeRate {
    ///汇率类型
    #[serde(rename = "type")]
    pub rate_type: Option<String>,

    /// 汇率值.
    pub rate: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RefundSourceRespInfo {
    /// 出资来源
    pub fund_source: String,

    /// 出资金额
    pub amount: u32,
}

/// 优惠退款详情
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DiscountedRefund {
    /// 券ID
    pub promotion_id: String,

    /// 优惠范围
    pub scope: Option<PromotionScope>,

    /// 优惠类型
    #[serde(rename = "type")]
    pub discount_type: Option<String>,

    /// 优惠券面额
    pub amount: Option<u32>,

    /// 优惠券退款额
    pub refund_amount: Option<u32>,

    /// 货币类型
    pub currency: Currency,
}
