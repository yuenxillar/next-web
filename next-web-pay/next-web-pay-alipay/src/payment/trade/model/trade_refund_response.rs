use serde::Deserialize;

use crate::Named;

/// 支付宝退款查询响应数据
#[derive(Debug, Clone, Deserialize)]
pub struct TradeRefundResponse {
    /// 支付宝交易号
    pub trade_no: String,

    /// 商户订单号
    pub out_trade_no: String,

    /// 用户的登录id
    pub buyer_logon_id: String,

    /// 退款总金额，单位：元
    pub refund_fee: String,

    /// 退款使用的资金渠道
    pub refund_detail_item_list: Option<Vec<TradeFundBill>>,

    /// 交易在支付时候的门店名称
    pub store_name: Option<String>,

    /// 买家在支付宝的用户id
    pub buyer_user_id: Option<String>,

    /// 买家支付宝用户唯一标识
    pub buyer_open_id: Option<String>,

    /// 本次商户实际退回金额，单位：元
    pub send_back_fee: Option<String>,

    /// 撤销的预授权金额，单位：元
    pub pre_auth_cancel_fee: Option<String>,

    /// 本次退款是否发生了资金变化
    pub fund_change: Option<String>,

    /// 本次请求退惠营宝金额，单位：元
    pub refund_hyb_amount: Option<String>,

    /// 退费信息
    pub refund_charge_info_list: Option<Vec<RefundChargeInfo>>,

    /// 退款使用的所有优惠券信息
    pub refund_voucher_detail_list: Option<Vec<VoucherDetail>>,
}

/// 交易支付使用的资金渠道
#[derive(Debug, Clone, Deserialize)]
pub struct TradeFundBill {
    /// 交易使用的资金渠道
    pub fund_channel: String,

    /// 该支付工具类型所使用的金额，单位：元
    pub amount: String,

    /// 渠道实际付款金额
    pub real_amount: Option<String>,

    /// 渠道所使用的资金类型
    pub fund_type: Option<FundType>,
}

/// 资金类型
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FundType {
    /// 借记卡
    DebitCard,
    /// 信用卡
    CreditCard,
    /// 借贷合一卡
    MixedCard,
}

/// 退费信息
#[derive(Debug, Clone, Deserialize)]
pub struct RefundChargeInfo {
    /// 实退费用，单位：元
    pub refund_charge_fee: Option<String>,

    /// 签约费率
    pub switch_fee_rate: Option<String>,

    /// 手续费类型：trade-收单手续费，hbfq-花呗分期手续，charge-其他手续费
    pub charge_type: Option<String>,

    /// 组合支付退费明细
    pub refund_sub_fee_detail_list: Option<Vec<RefundSubFee>>,
}

/// 组合支付退费明细
#[derive(Debug, Clone, Deserialize)]
pub struct RefundSubFee {
    /// 实退费用，单位：元
    pub refund_charge_fee: Option<String>,

    /// 签约费率
    pub switch_fee_rate: Option<String>,
}

/// 优惠券信息
#[derive(Debug, Clone, Deserialize)]
pub struct VoucherDetail {
    /// 券id（必填）
    pub id: String,

    /// 券名称（必填）
    pub name: String,

    /// 券类型（必填）
    #[serde(rename = "type")]
    pub voucher_type: String,

    /// 优惠券面额（必填），单位：元
    pub amount: String,

    /// 商家出资金额
    pub merchant_contribute: Option<String>,

    /// 其他出资方出资金额
    pub other_contribute: Option<String>,

    /// 优惠券备注信息
    pub memo: Option<String>,

    /// 券模板id
    pub template_id: Option<String>,

    /// 优惠券的其他出资方明细
    pub other_contribute_detail: Option<Vec<ContributeDetail>>,

    /// 用户购买券时实际付款金额
    pub purchase_buyer_contribute: Option<String>,

    /// 用户购买券时商户优惠金额
    pub purchase_merchant_contribute: Option<String>,

    /// 用户购买券时平台优惠金额
    pub purchase_ant_contribute: Option<String>,
}

/// 优惠券的其他出资方明细
#[derive(Debug, Clone, Deserialize)]
pub struct ContributeDetail {
    /// 出资方类型（必填）
    pub contribute_type: String,

    /// 出资方金额（必填），单位：元
    pub contribute_amount: String,
}

impl Named for TradeRefundResponse {
    fn name() -> &'static str {
        "alipay_trade_refund_response"
    }
}
