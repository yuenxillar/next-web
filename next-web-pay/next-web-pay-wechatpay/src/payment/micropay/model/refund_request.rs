use serde::Serialize;

use crate::Path;

#[derive(Debug, Clone, Default, Serialize)]
pub struct WechatPayRefundRequest {
    /// 商户号
    mchid: String,

    /// app ID.
    appid: String,

    /// 子商户号
    sub_mchid: String,

    /// 机构商户号
    sp_mchid: String,

    /// 机构APPID
    sp_appid: String,

    /// 子商户APPID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_appid: Option<String>,

    /// 微信订单号
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction_id: Option<String>,

    /// 商户订单号
    #[serde(skip_serializing_if = "Option::is_none")]
    out_trade_no: Option<String>,

    /// 商户退款单号
    out_refund_no: String,

    /// 退款原因
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// 退款资金来源
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// 订单金额
    amount: RefundReqAmount,

    /// 退款通知地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_url: Option<String>,
}

impl WechatPayRefundRequest {
    pub fn with_out_trade_no(
        mchid: impl Into<String>,
        appid: impl Into<String>,
        sp_mchid: impl Into<String>,
        sub_mchid: impl Into<String>,
        sp_appid: impl Into<String>,

        out_trade_no: impl Into<String>,
        out_refund_no: impl Into<String>,
        amount: RefundReqAmount,
    ) -> Self {
        Self {
            appid: appid.into(),
            mchid: mchid.into(),
            sub_mchid: sub_mchid.into(),
            sp_mchid: sp_mchid.into(),
            sp_appid: sp_appid.into(),
            out_trade_no: Some(out_trade_no.into()),
            out_refund_no: out_refund_no.into(),
            amount,
            ..Default::default()
        }
    }

    pub fn with_transaction_id(
        mchid: impl Into<String>,
        appid: impl Into<String>,
        sp_mchid: impl Into<String>,
        sub_mchid: impl Into<String>,
        sp_appid: impl Into<String>,

        transaction_id: impl Into<String>,
        out_refund_no: impl Into<String>,
        amount: RefundReqAmount,
    ) -> Self {
        Self {
            appid: appid.into(),
            mchid: mchid.into(),
            sub_mchid: sub_mchid.into(),
            sp_mchid: sp_mchid.into(),
            sp_appid: sp_appid.into(),
            transaction_id: Some(transaction_id.into()),
            out_refund_no: out_refund_no.into(),
            amount,
            ..Default::default()
        }
    }

    pub fn with_sub_appid(mut self, sub_appid: impl Into<String>) -> Self {
        self.sub_appid = Some(sub_appid.into());
        self
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_notify_url(mut self, notify_url: impl Into<String>) -> Self {
        self.notify_url = Some(notify_url.into());
        self
    }
}

/// 退款金额信息
#[derive(Debug, Clone, Default, Serialize)]
pub struct RefundReqAmount {
    /// 退款金额
    pub refund: u32,

    /// 原订单金额
    pub total: u32,

    /// 退款币种
    pub currency: String,

    /// 退款出资来源及金额
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Vec<RefundSourceInfo>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RefundSourceInfo {
    /// 出资来源
    ///
    /// FUNDS_REFUNDABLE_BALANCE : 可垫付退款余额
    /// ORDER_REFUNDABLE_BALANCE : 订单未分可退余额
    pub fund_source: Option<String>,

    /// 出资金额
    pub amount: u32,
}

impl Path for WechatPayRefundRequest {
    fn path() -> &'static str {
        "/v3/global/refunds"
    }
}
