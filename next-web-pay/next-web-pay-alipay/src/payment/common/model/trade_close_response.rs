use serde::Deserialize;

use crate::Named;

/// 支付宝退款查询响应数据
#[derive(Debug, Clone, Deserialize)]
pub struct AlipayTradeCloseResponse {
    /// 商户订单号
    pub out_trade_no: String,

    /// 支付宝交易号; 当发生交易关闭或交易退款时返回；
    pub trade_no: Option<String>,
}

impl Named for AlipayTradeCloseResponse {
    fn name() -> &'static str {
        "alipay_trade_close_response"
    }
}
