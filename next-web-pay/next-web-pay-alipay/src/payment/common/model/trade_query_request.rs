use serde::Serialize;

use crate::Method;

/// 统一收单交易查询
#[derive(Debug, Clone, Serialize)]
pub struct AlipayTradeQueryRequest {
    /// 订单支付时传入的商户订单号,和支付宝交易号不能同时为空。
    /// trade_no,out_trade_no如果同时存在优先取trade_no
    #[serde(skip_serializing_if = "Option::is_none")]
    out_trade_no: Option<String>,

    /// 支付宝交易号，和商户订单号不能同时为空
    #[serde(skip_serializing_if = "Option::is_none")]
    trade_no: Option<String>,

    /// 查询选项，商户通过上送该参数来定制同步需要额外返回的信息字段，数组格式。
    #[serde(skip_serializing_if = "Option::is_none")]
    query_options: Option<Vec<String>>,
}

impl AlipayTradeQueryRequest {
    pub fn with_out_trade_no(out_trade_no: String) -> Self {
        Self {
            out_trade_no: Some(out_trade_no),
            trade_no: None,
            query_options: None,
        }
    }

    pub fn with_trade_no(trade_no: String) -> Self {
        Self {
            out_trade_no: None,
            trade_no: Some(trade_no),
            query_options: None,
        }
    }

    pub fn with_query_options(mut self, query_options: Vec<String>) -> Self {
        self.query_options = Some(query_options);

        self
    }
}

impl Method for AlipayTradeQueryRequest {
    fn method() -> &'static str {
        "alipay.trade.query"
    }
}
