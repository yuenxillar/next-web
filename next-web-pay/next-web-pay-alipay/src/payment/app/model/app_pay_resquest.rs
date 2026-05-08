use serde::Serialize;

use crate::Method;

/// 支付宝支付请求参数
#[derive(Debug, Clone, Serialize)]
pub struct AlipayAppPayRequest {
    /// 商户网站唯一订单号
    /// 由商家自定义，64个字符以内，仅支持字母、数字、下划线且需保证在商户端不重复
    out_trade_no: String,

    /// 订单总金额，单位为元，精确到小数点后两位
    /// 取值范围[0.01,100000000]，金额不能为0
    total_amount: String,

    /// 订单标题
    /// 注意：不可使用特殊字符，如 /，=，& 等
    subject: String,

    /// 销售产品码，商家和支付宝签约的产品码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_code: Option<String>,

    /// 订单包含的商品列表信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_detail: Option<Vec<GoodsDetail>>,

    /// 绝对超时时间，格式为yyyy-MM-dd HH:mm:ss
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_expire: Option<String>,

    /// 业务扩展参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extend_params: Option<ExtendParams>,

    /// 公用回传参数
    /// 如果请求时传递了该参数，则会在支付结果异步通知中将该参数原样返回
    /// 本参数必须进行UrlEncode之后才可以发送给支付宝
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passback_params: Option<String>,

    /// 商户原始订单号，最大长度限制32位
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_order_no: Option<String>,

    /// 外部指定买家
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext_user_info: Option<ExtUserInfo>,

    /// 返回参数选项
    /// 商户通过传递该参数来定制同步需要额外返回的信息字段，数组格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_options: Option<Vec<String>>,
}

impl AlipayAppPayRequest {
    pub fn new(
        out_trade_no: impl Into<String>,
        total_amount: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            out_trade_no: out_trade_no.into(),
            total_amount: total_amount.into(),
            subject: subject.into(),
            product_code: None,
            goods_detail: None,
            time_expire: None,
            extend_params: None,
            passback_params: None,
            merchant_order_no: None,
            ext_user_info: None,
            query_options: None,
        }
    }
}

/// 商品明细
#[derive(Debug, Clone, Serialize)]
pub struct GoodsDetail {
    /// 商品的编号
    pub goods_id: String,

    /// 商品名称
    pub goods_name: String,

    /// 商品数量
    pub quantity: String,

    /// 商品单价，单位为元
    pub price: String,

    /// 支付宝定义的统一商品编号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alipay_goods_id: Option<String>,

    /// 商品类目
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_category: Option<String>,

    /// 商品类目树，从商品类目根节点到叶子节点的类目id组成，类目id值使用|分割
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories_tree: Option<String>,

    /// 商品的展示地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_url: Option<String>,
}

/// 业务扩展参数
#[derive(Debug, Clone, Serialize)]
pub struct ExtendParams {
    /// 系统商编号
    /// 该参数作为系统商返佣数据提取的依据，请填写系统商签约协议的PID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sys_service_provider_id: Option<String>,

    /// 使用花呗分期要进行的分期数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hb_fq_num: Option<String>,

    /// 使用花呗分期需要卖家承担的手续费比例的百分值，传入100代表100%
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hb_fq_seller_percent: Option<String>,

    /// 行业数据回流信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry_reflux_info: Option<String>,

    /// 卡类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_type: Option<String>,

    /// 是否进行资金冻结，用于后续分账
    /// true表示资金冻结，false或不传表示资金不冻结
    #[serde(skip_serializing_if = "Option::is_none")]
    pub royalty_freeze: Option<String>,
}

/// 外部指定买家信息
#[derive(Debug, Clone, Serialize)]
pub struct ExtUserInfo {
    /// 买家证件号
    /// 注：need_check_info=T时该参数才有效
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_no: Option<String>,

    /// 指定买家证件类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_type: Option<String>,

    /// 允许的最小买家年龄
    /// 买家年龄必须大于等于所传数值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_age: Option<String>,

    /// 指定买家手机号
    /// 注：该参数暂不校验
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,

    /// 指定买家姓名
    /// 注：need_check_info=T时该参数才有效
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// 是否强制校验买家信息
    /// 需要强制校验传：T; 不需要强制校验传：F或者不传
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_check_info: Option<String>,

    /// 买家加密身份信息
    /// 当指定了此参数且指定need_check_info=T时，支付宝会对买家身份进行校验
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_hash: Option<String>,
}

impl Method for AlipayAppPayRequest {
    fn method() -> &'static str {
        "alipay.trade.app.pay"
    }
}
