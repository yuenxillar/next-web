use serde::Deserialize;

use crate::Named;

/// 支付宝统一交易查询返回参数
#[derive(Debug, Clone, Deserialize)]
pub struct TradeQueryResponse {
    /// 支付宝交易号
    pub trade_no: String,

    /// 商家订单号
    pub out_trade_no: String,

    /// 交易状态
    pub trade_status: TradeStatus,

    /// 交易的订单金额，单位为元，两位小数
    pub total_amount: String,

    /// 交易支付使用的资金渠道
    pub fund_bill_list: Vec<TradeFundBill>,

    /// 买家在支付宝的用户id
    pub buyer_user_id: String,

    /// 本次交易打款给卖家的时间
    pub send_pay_date: Option<String>,

    /// 实收金额，单位为元，两位小数
    pub receipt_amount: Option<String>,

    /// 商户门店编号
    pub store_id: Option<String>,

    /// 商户机具终端编号
    pub terminal_id: Option<String>,

    /// 请求交易支付中的商户店铺的名称
    pub store_name: Option<String>,

    /// 买家支付宝用户唯一标识
    pub buyer_open_id: Option<String>,

    /// 平台优惠金额
    pub discount_amount: Option<String>,

    /// 交易额外信息，json格式
    pub ext_infos: Option<String>,

    /// 买家用户类型
    pub buyer_user_type: Option<BuyerUserType>,

    /// 商家优惠金额
    pub mdiscount_amount: Option<String>,

    /// 买家支付宝账号
    pub buyer_logon_id: Option<String>,

    /// 买家实付金额，单位为元，两位小数
    pub buyer_pay_amount: Option<String>,

    /// 交易中用户支付的可开具发票的金额，单位为元，两位小数
    pub invoice_amount: Option<String>,

    /// 积分支付的金额，单位为元，两位小数
    pub point_amount: Option<String>,
}

/// 交易状态
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradeStatus {
    /// 交易创建，等待买家付款
    WaitBuyerPay,
    /// 未付款交易超时关闭，或支付完成后全额退款
    TradeClosed,
    /// 交易支付成功
    TradeSuccess,
    /// 交易结束，不可退款
    TradeFinished,
}

/// 交易支付使用的资金渠道
#[derive(Debug, Clone, Deserialize)]
pub struct TradeFundBill {
    /// 交易使用的资金渠道
    pub fund_channel: String,

    /// 该支付工具类型所使用的金额
    pub amount: String,

    /// 渠道实际付款金额
    pub real_amount: Option<String>,
}

/// 买家用户类型
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuyerUserType {
    /// 企业用户
    Corporate,
    /// 个人用户
    Private,
}

impl Named for TradeQueryResponse {
    fn name() -> &'static str {
        "alipay_trade_query_response"
    }
}
