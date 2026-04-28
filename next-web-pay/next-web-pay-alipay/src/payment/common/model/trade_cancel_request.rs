use serde::Serialize;

use crate::Method;

/// 支付宝交易撤销请求参数
#[derive(Debug, Clone, Serialize)]
pub struct AlipayTradeCancelRequest {
    /// 原支付请求的商户订单号,和支付宝交易号不能同时为空
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,

    /// 支付宝交易号，和商户订单号不能同时为空
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_no: Option<String>,
}

impl AlipayTradeCancelRequest {
    pub fn with_out_trade_no(out_trade_no: impl Into<String>) -> Self {
        Self {
            out_trade_no: Some(out_trade_no.into()),
            trade_no: None,
        }
    }

    pub fn with_trade_no(trade_no: impl Into<String>) -> Self {
        Self {
            out_trade_no: None,
            trade_no: Some(trade_no.into()),
        }
    }
}

impl Method for AlipayTradeCancelRequest {
    fn method() -> &'static str {
        "alipay.trade.cancel"
    }
}
