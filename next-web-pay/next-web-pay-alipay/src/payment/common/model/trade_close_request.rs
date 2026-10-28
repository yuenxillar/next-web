use serde::Serialize;

use crate::Method;

#[derive(Debug, Clone, Serialize)]
pub struct AlipayTradeCloseRequest {
    /// 原支付请求的商户订单号,和支付宝交易号不能同时为空
    pub out_trade_no: Option<String>,

    /// 支付宝交易号，和商户订单号不能同时为空
    pub trade_no: Option<String>,

    /// 商家操作员编号 id，由商家自定义
    pub operator_id: Option<String>,
}

impl AlipayTradeCloseRequest {
    pub fn with_out_trade_no(out_trade_no: impl Into<String>) -> Self {
        Self {
            out_trade_no: Some(out_trade_no.into()),
            trade_no: None,
            operator_id: None,
        }
    }

    pub fn with_trade_no(trade_no: impl Into<String>) -> Self {
        Self {
            out_trade_no: None,
            trade_no: Some(trade_no.into()),
            operator_id: None,
        }
    }

    pub fn with_operator_id(mut self, operator_id: impl Into<String>) -> Self {
        self.operator_id = Some(operator_id.into());

        self
    }
}

impl Method for AlipayTradeCloseRequest {
    fn method() -> &'static str {
        "alipay.trade.close"
    }
}
