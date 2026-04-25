use serde::Deserialize;

use crate::Named;

/// 支付宝退款查询响应数据
#[derive(Debug, Clone, Deserialize)]
pub struct TradeRefundQueryResponse {
    /// 支付宝交易号
    pub trade_no: Option<String>,

    /// 创建交易传入的商户订单号
    pub out_trade_no: Option<String>,

    /// 本笔退款对应的退款请求号
    pub out_request_no: Option<String>,

    /// 该笔退款所对应的交易的订单金额，单位：元
    pub total_amount: Option<String>,

    /// 本次退款请求对应的退款金额，单位：元
    pub refund_amount: Option<String>,

    /// 退款状态
    pub refund_status: Option<RefundStatus>,

    /// 退分账明细信息，当前仅在直付通产品中返回
    pub refund_royaltys: Option<Vec<RefundRoyaltyResult>>,

    /// 退款时间
    pub gmt_refund_pay: Option<String>,

    /// 本次退款使用的资金渠道
    pub refund_detail_item_list: Option<Vec<TradeFundBill>>,

    /// 本次商户实际退回金额，单位：元
    pub send_back_fee: Option<String>,

    /// 银行卡冲退信息
    pub deposit_back_info: Option<DepositBackInfo>,

    /// 优惠券信息
    pub refund_voucher_detail_list: Option<Vec<VoucherDetail>>,

    /// 撤销的预授权金额，单位：元
    pub pre_auth_cancel_fee: Option<String>,

    /// 本次退款金额中退惠营宝的金额，单位：元
    pub refund_hyb_amount: Option<String>,

    /// 退费信息
    pub refund_charge_info_list: Option<Vec<RefundChargeInfo>>,

    /// 银行卡冲退信息列表
    pub deposit_back_info_list: Option<Vec<DepositBackInfo>>,
}

/// 退款状态
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    /// 退款处理成功
    RefundSuccess,
}

/// 退分账明细信息
#[derive(Debug, Clone, Deserialize)]
pub struct RefundRoyaltyResult {
    /// 退分账金额，单位：元（必填）
    pub refund_amount: String,

    /// 退分账结果码（必填）
    pub result_code: String,

    /// 分账类型
    pub royalty_type: Option<RoyaltyType>,

    /// 转出人支付宝账号对应用户ID
    pub trans_out: Option<String>,

    /// 转出人支付宝账号
    pub trans_out_email: Option<String>,

    /// 转入人支付宝账号对应用户ID
    pub trans_in: Option<String>,

    /// 转入人支付宝账号
    pub trans_in_email: Option<String>,

    /// 商户请求的转出账号
    pub ori_trans_out: Option<String>,

    /// 商户请求的转入账号
    pub ori_trans_in: Option<String>,
}

/// 分账类型
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoyaltyType {
    /// 普通分账类型
    Transfer,
    /// 补差分账类型
    Replenish,
}

/// 银行卡冲退信息
#[derive(Debug, Clone, Deserialize)]
pub struct DepositBackInfo {
    /// 是否存在银行卡冲退信息
    pub has_deposit_back: Option<String>,

    /// 银行卡冲退状态：S-成功，F-失败，P-处理中
    pub dback_status: Option<String>,

    /// 银行卡冲退金额，单位：元
    pub dback_amount: Option<String>,

    /// 银行响应时间，格式为yyyy-MM-dd HH:mm:ss
    pub bank_ack_time: Option<String>,

    /// 预估银行到账时间，格式为yyyy-MM-dd HH:mm:ss
    pub est_bank_receipt_time: Option<String>,
}

/// 交易支付使用的资金渠道
#[derive(Debug, Clone, Deserialize)]
pub struct TradeFundBill {
    /// 交易使用的资金渠道（必填）
    pub fund_channel: String,

    /// 该支付工具类型所使用的金额，单位：元（必填）
    pub amount: String,

    /// 渠道实际付款金额，单位：元
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

    /// 优惠券面额，单位：元（必填）
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

    /// 出资方金额，单位：元（必填）
    pub contribute_amount: String,
}

impl Named for TradeRefundQueryResponse {
    fn name() -> &'static str {
        "alipay_trade_fastpay_refund_query_response"
    }
}
