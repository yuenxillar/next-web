use serde::Deserialize;

/// WeChat Pay V3 Global MicroPay response.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WechatPayMicroPayResponse {
    /// 微信支付订单号
    pub id: String,

    /// 商户号
    pub mchid: String,

    /// APPID
    pub appid: String,

    /// 子商户号
    pub sub_mchid: String,

    /// 机构商户号
    pub sp_mchid: String,

    /// 机构APPID
    pub sp_appid: String,

    /// 子商户APPID
    pub sub_appid: Option<String>,

    /// 商户订单号
    pub out_trade_no: String,

    /// 交易类型
    pub trade_type: String,

    /// 交易状态
    ///
    /// SUCCESS, REFUND, NOTPAY, CLOSED, REVOKED, USERPAYING, PAYERROR.
    pub trade_state: String,

    /// 交易状态描述
    pub trade_state_desc: String,

    /// 付款银行
    pub bank_type: String,

    /// 商户数据
    pub attach: Option<String>,

    /// 支付完成时间
    pub success_time: String,

    /// 支付者
    pub payer: MicroPayResponsePayer,

    /// Order amount info.
    pub amount: MicroPayResponseAmount,

    /// Promotion detail list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promotion_detail: Option<MicroPayPromotionDetail>,
}

/// Response payer info.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MicroPayResponsePayer {
    /// 用户标识
    ///
    /// 用户在商户appid对应下的唯一标识，需要传appid才有返回
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openid: Option<String>,

    /// 用户标识（机构）
    ///
    /// 用户在机构sp_appid对应下的唯一标识，openid和sub_openid可以选传其中之一，如果选择传sub_openid,则必须传sub_appid
    /// 下单前需要调用【网页授权】接口获取到用户的openid。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sp_openid: Option<String>,

    /// 用户标识（子商户）
    ///
    /// 用户在子商户sub_appid下用户唯一标识，openid和sub_openid可以选传其中之一，如果选择传sub_openid,则必须传sub_appid
    /// 下单前需要调用【网页授权】接口获取到用户的openid，
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_openid: Option<String>,
}

/// Response amount info.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MicroPayResponseAmount {
    /// Total order amount in the smallest currency unit.
    pub total: u32,

    /// ISO 4217 three-letter currency code.
    pub currency: Option<String>,

    /// Actual amount paid by the user.
    pub payer_total: u32,

    /// Payment currency code.
    pub payer_currency: Option<String>,

    /// Exchange rate info.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<MicroPayExchangeRate>,
}

/// Exchange rate info.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MicroPayExchangeRate {
    ///汇率类型
    #[serde(rename = "type")]
    pub rate_type: Option<String>,

    /// 汇率值.
    pub rate: Option<u32>,
}

/// Promotion detail.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MicroPayPromotionDetail {
    /// 券或者立减优惠id
    pub promotion_id: String,

    /// 优惠名称
    pub name: Option<String>,

    /// 优惠范围
    ///
    /// GLOBAL：全场代金券
    /// SINGLE：单品优惠
    pub scope: Option<String>,

    /// 优惠类型
    ///
    /// COUPON- 代金券，需要走结算资金的充值型代金券,（境外商户券币种与支付币种一致）
    /// DISCOUNT- 优惠券，不走结算资金的免充值型优惠券，（境外商户券币种与标价币种一致
    #[serde(rename = "type")]
    pub promotion_type: Option<String>,

    /// 用户享受优惠的金额
    pub amount: u32,

    /// 货币类型
    pub currency: Option<String>,

    /// 活动ID
    pub activity_id: Option<String>,

    /// 特指由微信支付商户平台创建的优惠，出资金额等于本项优惠总金额
    pub wechatpay_contribute_amount: Option<u32>,

    /// 特指商户自己创建的优惠，出资金额等于本项优惠总金额
    pub merchant_contribute_amount: Option<u32>,

    /// 其他出资方出资金额
    pub other_contribute_amount: Option<u32>,

    /// 单品列表
    pub goods_detail: Vec<MicroPayPromotionGoodsDetail>,
}

/// Goods detail within a promotion.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MicroPayPromotionGoodsDetail {
    /// 商品编码
    ///
    /// 由半角的大小写字母、数字、中划线、下划线中的一种或几种组成
    pub goods_id: String,

    /// 商品备注
    ///
    /// goods_remark为备注字段，按照配置原样返回，字段内容在微信后台配置券时进行设置。
    pub goods_remark: Option<String>,

    /// 商品数量
    ///
    /// 用户购买的数量
    pub quantity: u32,

    /// 商品价格
    ///
    /// 单位为：分。如果商户有优惠，需传输商户优惠后的单价(例如：用户对一笔100元的订单使用了商场发的纸质优惠券100-50，则活动商品的单价应为原单价-50
    pub price: u32,
}

/// Error detail.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MicroPayErrorDetail {
    /// JSON Pointer to the problematic field.
    pub field: Option<String>,
    /// Value of the problematic field.
    pub value: Option<String>,
    /// Specific error reason.
    pub issue: Option<String>,
    /// Location: body, url, or query.
    pub location: Option<String>,
}
