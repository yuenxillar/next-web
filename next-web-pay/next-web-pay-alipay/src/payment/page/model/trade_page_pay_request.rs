use serde::{Deserialize, Serialize};

use crate::Method;

/// 支付宝统一收单线下交易预创建请求参数
#[derive(Debug, Clone, Serialize)]
pub struct AlipayTradePagePayRequest {
    /// 商户订单号 (必选)
    /// 64个字符以内，仅支持字母、数字、下划线且需保证在商户端不重复
    out_trade_no: String,

    /// 订单总金额 (必选)
    /// 单位为元，精确到小数点后两位，取值范围为 [0.01,100000000]
    total_amount: String,

    /// 订单标题 (必选)
    /// 不可使用特殊字符，如 /，=，& 等
    subject: String,

    /// 销售产品码 (必选)
    product_code: String,

    /// PC扫码支付的方式 (可选)
    /// 0：订单码-简约前置模式
    /// 1：订单码-前置模式
    /// 3：订单码-迷你前置模式
    /// 4：订单码-可定义宽度的嵌入式二维码
    /// 2：订单码-跳转模式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_pay_mode: Option<String>,

    /// 商户自定义二维码宽度 (可选)
    /// qr_pay_mode=4时该参数有效
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qrcode_width: Option<i32>,

    /// 订单包含的商品列表信息 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_detail: Option<Vec<GoodsDetail>>,

    /// 订单绝对超时时间 (可选)
    /// 格式为yyyy-MM-dd HH:mm:ss
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_expire: Option<String>,

    /// 二级商户信息 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_merchant: Option<SubMerchant>,

    /// 业务扩展参数 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extend_params: Option<ExtendParams>,

    /// 商户传入业务信息 (可选)
    /// 格式为json格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_params: Option<String>,

    /// 优惠参数 (可选)
    /// 为 JSON 格式，仅与支付宝协商后可用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promo_params: Option<String>,

    /// 请求后页面的集成方式 (可选)
    /// ALIAPP：支付宝钱包内
    /// PCWEB：PC端访问
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_type: Option<String>,

    /// 请求来源地址 (可选)
    /// 如果使用ALIAPP的集成方式，用户中途取消支付会返回该地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_from_url: Option<String>,

    /// 商户门店编号 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_id: Option<String>,

    /// 商户原始订单号 (可选)
    /// 最大长度限制 32 位
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_order_no: Option<String>,

    /// 外部指定买家 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext_user_info: Option<ExtUserInfo>,

    /// 开票信息 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_info: Option<InvoiceInfo>,

    #[serde(skip)]
    pub(crate) return_url: Option<String>,
}

impl AlipayTradePagePayRequest {
    pub fn new(
        out_trade_no: impl Into<String>,
        total_amount: impl Into<String>,
        subject: impl Into<String>,
        return_url: impl Into<String>,
    ) -> Self {
        Self {
            out_trade_no: out_trade_no.into(),
            total_amount: total_amount.into(),
            subject: subject.into(),
            product_code: "FAST_INSTANT_TRADE_PAY".into(),
            return_url: Some(return_url.into()),
            qr_pay_mode: Default::default(),
            qrcode_width: Default::default(),
            goods_detail: Default::default(),
            time_expire: Default::default(),
            sub_merchant: Default::default(),
            extend_params: Default::default(),
            business_params: Default::default(),
            promo_params: Default::default(),
            integration_type: Default::default(),
            request_from_url: Default::default(),
            store_id: Default::default(),
            merchant_order_no: Default::default(),
            ext_user_info: Default::default(),
            invoice_info: Default::default(),
        }
    }

    pub fn with_product_code(mut self, product_code: impl Into<String>) -> Self {
        self.product_code = product_code.into();
        self
    }
}

/// 商品详情
#[derive(Debug, Clone, Serialize)]
pub struct GoodsDetail {
    /// 商品的编号 (必选)
    pub goods_id: String,

    /// 商品名称 (必选)
    pub goods_name: String,

    /// 商品数量 (必选)
    pub quantity: u32,

    /// 商品单价，单位为元 (必选)
    pub price: String,

    /// 支付宝定义的统一商品编号 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alipay_goods_id: Option<String>,

    /// 商品类目 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_category: Option<String>,

    /// 商品类目树 (可选)
    /// 从商品类目根节点到叶子节点的类目id组成，类目id值使用|分割
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories_tree: Option<String>,

    /// 商品的展示地址 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_url: Option<String>,
}

/// 二级商户信息
#[derive(Debug, Clone, Serialize)]
pub struct SubMerchant {
    /// 间连受理商户的支付宝商户编号 (必选)
    pub merchant_id: String,

    /// 二级商户编号类型 (可选)
    /// 枚举值：alipay
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_type: Option<String>,
}

/// 业务扩展参数
#[derive(Debug, Clone, Serialize)]
pub struct ExtendParams {
    /// 系统商编号 (可选)
    /// 该参数作为系统商返佣数据提取的依据，请填写系统商签约协议的PID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sys_service_provider_id: Option<String>,

    /// 使用花呗分期要进行的分期数 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hb_fq_num: Option<String>,

    /// 使用花呗分期需要卖家承担的手续费比例的百分值 (可选)
    /// 传入100代表100%
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hb_fq_seller_percent: Option<String>,

    /// 行业数据回流信息 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry_reflux_info: Option<String>,

    /// 卡类型 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_type: Option<String>,

    /// 是否进行资金冻结，用于后续分账 (可选)
    /// true表示资金冻结，false或不传表示资金不冻结
    #[serde(skip_serializing_if = "Option::is_none")]
    pub royalty_freeze: Option<String>,
}

/// 外部指定买家
#[derive(Debug, Clone, Serialize)]
pub struct ExtUserInfo {
    /// 买家证件号 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_no: Option<String>,

    /// 指定买家证件类型 (可选)
    /// IDENTITY_CARD：身份证
    /// PASSPORT：护照
    /// OFFICER_CARD：军官证
    /// SOLDIER_CARD：士兵证
    /// HOKOU：户口本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_type: Option<CertType>,

    /// 允许的最小买家年龄 (可选)
    /// 买家年龄必须大于等于所传数值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_age: Option<String>,

    /// 指定买家手机号 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,

    /// 指定买家姓名 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// 是否强制校验买家信息 (可选)
    /// T: 强制校验; F或不传: 不校验
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_check_info: Option<String>,

    /// 买家加密身份信息 (可选)
    /// 当指定了此参数且指定need_check_info=T时，支付宝会对买家身份进行校验
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize,PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CertType {
    /// 身份证
    IdentityCard,

    /// 护照
    #[serde(rename = "PASSPORT")]
    PassPort,

    ///军官证
    OfficerCard,

    /// 士兵证
    SoldierCard,

    ///户口本
    HoKou,

    /// 其他证件
    #[serde(other)]
    Other,
}

impl std::fmt::Display for CertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CertType::IdentityCard => "IDENTITY_CARD",
            CertType::PassPort => "PASSPORT",
            CertType::OfficerCard => "OFFICER_CARD",
            CertType::SoldierCard => "SOLDIER_CARD",
            CertType::HoKou => "HOKOU",
            CertType::Other => "OTHER",
        };
        write!(f, "{}", s)
    }
}

/// 开票信息
#[derive(Debug, Clone, Serialize)]
pub struct InvoiceInfo {
    /// 开票关键信息 (必选)
    pub key_info: InvoiceKeyInfo,

    /// 开票内容
    /// Json 数组格式
    pub details: String,
}

/// 开票关键信息
#[derive(Debug, Clone, Serialize)]
pub struct InvoiceKeyInfo {
    /// 该交易是否支持开票 (必选)
    pub is_support_invoice: bool,

    /// 开票商户名称：商户品牌简称|商户门店简称 (必选)
    pub invoice_merchant_name: String,

    /// 税号 (必选)
    pub tax_num: String,
}

/// 开票内容项
#[derive(Debug, Clone, Serialize)]
pub struct InvoiceDetail {
    /// 商品编码
    pub code: String,

    /// 商品名称
    pub name: String,

    /// 数量
    pub num: String,

    /// 总金额
    pub sum_price: String,

    /// 税率
    pub tax_rate: String,
}

impl Method for AlipayTradePagePayRequest {
    fn method() -> &'static str {
        "alipay.trade.page.pay"
    }
}
