use serde::Serialize;

use crate::Method;

use crate::payment::page::model::CertType;
use serde::Deserialize;

/// 支付宝手机网站支付请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AlipayTradeWapPayRequest {
    /// 商户网站唯一订单号（必填）
    pub out_trade_no: String,
    /// 订单总金额（必填），单位元，精确到小数点后两位
    /// 例如：9.00
    pub total_amount: String,
    /// 订单标题（必填）
    pub subject: String,
    /// 销售产品码（必填），手机网站支付为：QUICK_WAP_WAY
    pub product_code: String,
    /// 针对用户授权接口，获取用户相关数据时，用于标识用户授权关系
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    /// 用户付款中途退出返回商户网站的地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quit_url: Option<String>,
    /// 订单包含的商品列表信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_detail: Option<Vec<GoodsDetail>>,
    /// 绝对超时时间，格式为yyyy-MM-dd HH:mm:ss
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_expire: Option<String>,
    /// 业务扩展参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extend_params: Option<ExtendParams>,
    /// 商户传入业务信息，具体值要和支付宝约定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_params: Option<String>,
    /// 公用回传参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passback_params: Option<String>,
    /// 商户原始订单号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_order_no: Option<String>,
    /// 外部指定买家
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext_user_info: Option<ExtUserInfo>,
}

impl AlipayTradeWapPayRequest {
    /// 创建新的支付请求（必填参数）
    pub fn new(
        out_trade_no: impl Into<String>,
        total_amount: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            out_trade_no: out_trade_no.into(),
            total_amount: total_amount.into(),
            subject: subject.into(),
            product_code: "QUICK_WAP_WAY".to_string(),
            auth_token: None,
            quit_url: None,
            goods_detail: None,
            time_expire: None,
            extend_params: None,
            business_params: None,
            passback_params: None,
            merchant_order_no: None,
            ext_user_info: None,
        }
    }

    /// 验证必填参数
    pub fn validate(&self) -> Result<(), String> {
        if self.out_trade_no.is_empty() {
            return Err("out_trade_no 不能为空".to_string());
        }
        if self.out_trade_no.len() > 64 {
            return Err("out_trade_no 长度不能超过64".to_string());
        }
        // if self.total_amount < 0.01 || self.total_amount > 100_000_000.0 {
        //     return Err("total_amount 必须在 [0.01, 100000000] 范围内".to_string());
        // }
        if self.subject.is_empty() {
            return Err("subject 不能为空".to_string());
        }
        if self.subject.len() > 256 {
            return Err("subject 长度不能超过256".to_string());
        }

        // 检查 subject 中的特殊字符
        let invalid_chars = ['/', '=', '&'];
        if self.subject.chars().any(|c| invalid_chars.contains(&c)) {
            return Err("subject 不能包含特殊字符 /, =, &".to_string());
        }

        Ok(())
    }
}

/// 商品明细
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GoodsDetail {
    /// 商品的编号
    pub goods_id: String,
    /// 商品名称
    pub goods_name: String,
    /// 商品数量
    pub quantity: u32,
    /// 商品单价，单位为元
    pub price: String,
    /// 支付宝定义的统一商品编号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alipay_goods_id: Option<String>,
    /// 商品类目
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_category: Option<String>,
    /// 商品类目树
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories_tree: Option<String>,
    /// 商品描述信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// 商品的展示地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_url: Option<String>,
}

/// 业务扩展参数
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendParams {
    /// 系统商编号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sys_service_provider_id: Option<String>,
    /// 使用花呗分期要进行的分期数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hb_fq_num: Option<String>,
    /// 使用花呗分期需要卖家承担的手续费比例的百分值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hb_fq_seller_percent: Option<String>,
    /// 行业数据回流信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry_reflux_info: Option<String>,
    /// 卡类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_type: Option<String>,
    /// 是否进行资金冻结，用于后续分账
    #[serde(skip_serializing_if = "Option::is_none")]
    pub royalty_freeze: Option<bool>,
}

/// 外部指定买家信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtUserInfo {
    /// 指定买家姓名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 指定买家手机号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<String>,
    /// 指定买家证件类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_type: Option<CertType>,
    /// 买家证件号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_no: Option<String>,
    /// 允许的最小买家年龄
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_age: Option<u8>,
    /// 是否强制校验买家身份
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_buyer: Option<bool>,
    /// 是否强制校验买家信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_check_info: Option<bool>,
    /// 买家加密身份信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_hash: Option<String>,
}

impl Method for AlipayTradeWapPayRequest {
    fn method() -> &'static str {
        "alipay.trade.wap.pay"
    }
}
