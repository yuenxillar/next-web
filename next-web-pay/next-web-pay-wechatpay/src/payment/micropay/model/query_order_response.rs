use serde::{Deserialize, Serialize};

use crate::payment::model::common::{BankType, TradeState};

/// 微信支付订单查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatPayQueryOrderResponse {
    /// 商户号
    pub mchid: String,

    /// 商户APPID
    pub appid: String,

    /// 机构商户号
    pub sp_mchid: String,

    /// 子商户号
    pub sub_mchid: String,

    /// 机构APPID
    pub sp_appid: String,

    /// 子商户APPID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_appid: Option<String>,

    /// 商户订单号
    pub out_trade_no: String,

    /// 微信支付订单号
    pub id: String,

    /// 商户数据（附加数据）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<String>,

    /// 交易类型
    pub trade_type: TradeType,

    /// 付款银行
    pub bank_type: BankType,

    /// 支付完成时间（RFC3339格式）
    pub success_time: String,

    /// 交易状态
    pub trade_state: TradeState,

    /// 交易状态描述
    pub trade_state_desc: String,

    /// 支付者信息
    pub payer: Payer,

    /// 订单金额
    pub amount: Amount,

    /// 优惠详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promotion_detail: Option<Vec<PromotionDetail>>,
}

/// 支付者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payer {
    /// 用户标识
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openid: Option<String>,

    /// 用户标识
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sp_openid: Option<String>,

    /// 用户标识（子商户）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_openid: Option<String>,
}

/// 订单金额
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    /// 订单总金额（单位：分）
    pub total: u32,

    /// 货币类型（ISO 4217）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// 用户支付金额（单位：分）
    pub payer_total: u32,

    /// 支付货币类型（ISO 4217）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_currency: Option<String>,

    /// 汇率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<ExchangeRate>,
}

/// 汇率信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    /// 汇率（例如：6.5 表示 1美元=6.5人民币）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,

    /// 源货币
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_currency: Option<String>,

    /// 目标货币
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_currency: Option<String>,
}

/// 优惠详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionDetail {
    /// 券ID
    pub promotion_id: String,

    /// 优惠名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// 优惠范围
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<PromotionScope>,

    /// 优惠类型
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub promotion_type: Option<PromotionType>,

    /// 优惠券面额（单位：分）
    pub amount: u32,

    /// 货币类型（ISO 4217）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// 活动ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_id: Option<String>,

    /// 微信出资（单位：分）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wechatpay_contribute_amount: Option<u32>,

    /// 商户出资（单位：分）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_contribute_amount: Option<u32>,

    /// 其他出资（单位：分）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_contribute_amount: Option<u32>,

    /// 单品列表
    pub goods_detail: Vec<GoodsDetail>,
}

/// 单品信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsDetail {
    /// 商品编码
    pub goods_id: String,

    /// 商品数量
    pub quantity: u32,

    /// 商品单价（单位：分）
    pub price: u32,

    /// 商品优惠金额（单位：分）
    pub discount_amount: u32,

    /// 商品备注
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_remark: Option<String>,
}

/// 交易类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TradeType {
    /// APP支付
    App,
    /// JSAPI支付（公众号/小程序）
    Jsapi,
    /// Native支付
    Native,
    /// H5支付
    Mweb,
    /// 小程序支付
    #[serde(rename = "MINIPROGRAM")]
    MiniProgram,
    /// 刷卡支付
    #[serde(rename = "MICROPAY")]
    Micropay,
}

/// 优惠范围
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PromotionScope {
    /// 全场代金券
    Global,
    /// 单品优惠
    Single,
}

/// 优惠类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PromotionType {
    /// 代金券
    Coupon,
    /// 优惠券
    Discount,
}
