use serde::Serialize;

/// 统一收单交易退款查询
#[derive(Debug, Clone, Serialize)]
pub struct TradeRefundQueryRequest {
    /// 退款请求号。 请求退款接口时，传入的退款请求号，如果在退款请求时未传入，则该值为创建交易时的商户订单号
    out_request_no: String,

    /// 支付宝交易号。 和商户订单号不能同时为空
    #[serde(skip_serializing_if = "Option::is_none")]
    trade_no: Option<String>,

    /// 商户订单号。 订单支付时传入的商户订单号,和支付宝交易号不能同时为空
    #[serde(skip_serializing_if = "Option::is_none")]
    out_trade_no: Option<String>,

    /// 查询选项，商户通过上送该参数来定制同步需要额外返回的信息字段，数组格式。
    #[serde(skip_serializing_if = "Option::is_none")]
    query_options: Option<Vec<String>>,
}

impl TradeRefundQueryRequest {
    pub fn with_out_trade_no(
        out_request_no: impl Into<String>,
        out_trade_no: impl Into<String>,
    ) -> Self {
        let out_request_no = out_request_no.into();
        let out_trade_no = out_trade_no.into();

        Self {
            out_request_no,
            trade_no: None,
            out_trade_no: Some(out_trade_no),
            query_options: None,
        }
    }

    pub fn with_trade_no(out_request_no: impl Into<String>, trade_no: impl Into<String>) -> Self {
        let out_request_no = out_request_no.into();
        let trade_no = trade_no.into();

        Self {
            out_request_no,
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
