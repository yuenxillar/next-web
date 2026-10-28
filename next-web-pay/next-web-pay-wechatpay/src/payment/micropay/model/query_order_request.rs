use form_urlencoded::Serializer;
use serde::Serialize;

use crate::{ToPath, error::WechatPayError};

/// 查询订单请求参数
#[derive(Debug, Clone, Default, Serialize)]
pub struct WechatPayQueryOrderRequest {
    /// 商户号
    mchid: String,

    /// 子商户号
    sub_mchid: String,

    /// 机构商户号
    sp_mchid: String,

    /// 商户订单号
    #[serde(skip_serializing_if = "Option::is_none")]
    out_trade_no: Option<String>,

    /// 微信支付订单号
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
}

impl WechatPayQueryOrderRequest {
    pub fn new(
        mchid: impl Into<String>,
        sub_mchid: impl Into<String>,
        sp_mchid: impl Into<String>,
        out_trade_no: Option<String>,
        id: Option<String>,
    ) -> Self {
        Self {
            mchid: mchid.into(),
            sub_mchid: sub_mchid.into(),
            sp_mchid: sp_mchid.into(),
            out_trade_no,
            id,
        }
    }
}

impl ToPath for WechatPayQueryOrderRequest {
    fn to_path(&self) -> Result<String, WechatPayError> {
        let mut serializer = Serializer::new(String::new());
        serializer.append_pair("mchid", &self.mchid);
        serializer.append_pair("sub_mchid", &self.sub_mchid);
        serializer.append_pair("sp_mchid", &self.sp_mchid);

        let query = serializer.finish();

        match self.out_trade_no {
            Some(ref out_trade_no) => Ok(format!(
                "/v3/global/transactions/out-trade-no/{out_trade_no}?{query}",
            )),
            None => match self.id {
                Some(ref id) => Ok(format!("/v3/global/transactions/{id}?{query}")),
                None => Err(WechatPayError::MissingField("out_trade_no")),
            },
        }
    }
}
