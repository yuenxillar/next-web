use serde::Deserialize;

use crate::Named;

#[derive(Debug, Clone, Deserialize)]
pub struct AlipayTradePrecreateResponse {
    /// 商户订单号
    pub out_trade_no: String,

    /// 当前预下单请求生成的二维码码串，有效时间2小时，可以用二维码生成工具根据该码串值生成对应的二维码
    pub qr_code: Option<String>,
}

impl Named for AlipayTradePrecreateResponse {
    fn name() -> &'static str {
        "alipay_trade_precreate_response"
    }
}
