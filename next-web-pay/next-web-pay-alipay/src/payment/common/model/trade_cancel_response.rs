use serde::Deserialize;

use crate::Named;

///交易撤销
#[derive(Debug, Clone, Deserialize)]
pub struct AlipayTradeCancelResponse {
    /// 商户订单号
    pub out_trade_no: String,

    /// 是否需要重试
    pub retry_flag: String,

    /// 支付宝交易号; 当发生交易关闭或交易退款时返回；
    pub trade_no: Option<String>,

    /// 本次撤销触发的交易动作,接口调用成功且交易存在时返回。可能的返回值： close：交易未支付，触发关闭交易动作，无退款；
    /// refund：交易已支付，触发交易退款动作； 未返回：未查询到交易，或接口调用失败；
    pub action: Option<String>,
}

impl Named for AlipayTradeCancelResponse {
    fn name() -> &'static str {
        "alipay_trade_cancel_response"
    }
}
