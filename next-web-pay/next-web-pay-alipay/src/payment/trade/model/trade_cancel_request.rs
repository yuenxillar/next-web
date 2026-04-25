use serde::Deserialize;

/// 支付宝交易撤销请求参数
#[derive(Debug, Clone, Deserialize)]
pub struct TradeCancelRequest {
    /// 原支付请求的商户订单号,和支付宝交易号不能同时为空
    pub out_trade_no: Option<String>,

    /// 支付宝交易号，和商户订单号不能同时为空
    pub trade_no: Option<String>,
}

impl TradeCancelRequest {
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
