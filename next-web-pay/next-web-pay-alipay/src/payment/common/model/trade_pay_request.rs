use serde::{Deserialize, Serialize};

use crate::Method;

/// 支付宝当面付请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlipayTradePayRequest {
    /// 商户订单号（必填）
    /// 64个字符以内，仅支持字母、数字、下划线且需保证在商户端不重复
    out_trade_no: String,

    /// 订单总金额（必填）
    /// 单位为元，精确到小数点后两位，取值范围：[0.01, 100000000]
    total_amount: String, // 使用 String 保持精确，或用 Decimal

    /// 订单标题（必填）
    /// 256个字符以内，不可使用特殊字符如 /, =, & 等
    subject: String,

    /// 支付授权码（必填）
    /// 当面付场景传买家的付款码（25~30开头的长度为16~24位的数字）
    /// 或刷脸标识串（fp开头的35位字符串）
    auth_code: String,

    /// 支付场景（必填）
    /// bar_code: 当面付条码支付场景
    /// security_code: 当面付刷脸支付场景
    scene: PaymentScene,

    /// 产品码（可选）
    /// 当面付快捷版: OFFLINE_PAYMENT
    /// 其他当面付产品: FACE_TO_FACE_PAYMENT
    /// 默认: FACE_TO_FACE_PAYMENT
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_code: Option<ProductCode>,

    /// 卖家支付宝用户ID（可选）
    /// 28个字符，需要指定收款账号时传入
    /// 示例: 2088102146225135
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_id: Option<String>,

    /// 订单包含的商品列表信息（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_detail: Option<Vec<GoodsDetail>>,

    /// 业务扩展参数（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extend_params: Option<ExtendParams>,

    /// 商户传入业务信息（可选）
    /// 应用于安全、营销等参数直传场景
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_params: Option<BusinessParams>,

    /// 优惠明细参数（可选）
    /// 仅与支付宝协商后可用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promo_params: Option<PromoParam>,

    /// 商户门店编号（可选）
    /// 32个字符，商户创建门店时输入的门店编号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_id: Option<String>,

    /// 商户操作员编号（可选）
    /// 28个字符
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<String>,

    /// 商户机具终端编号（可选）
    /// 32个字符
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,

    /// 返回参数选项（可选）
    /// 定制同步需要额外返回的信息字段
    /// 如: ["fund_bill_list", "voucher_detail_list", "discount_goods_detail"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_options: Option<Vec<QueryOption>>,
}

/// 支付场景枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentScene {
    /// 当面付条码支付场景
    BarCode,
    /// 当面付刷脸支付场景
    SecurityCode,
}

impl Default for PaymentScene {
    fn default() -> Self {
        Self::BarCode
    }
}

/// 产品码枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProductCode {
    /// 当面付快捷版
    OfflinePayment,
    /// 其他当面付产品
    FaceToFacePayment,
}

impl Default for ProductCode {
    fn default() -> Self {
        Self::FaceToFacePayment
    }
}

/// 商品详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsDetail {
    /// 商品编号（必填）
    pub goods_id: String,

    /// 商品名称（必填）
    pub goods_name: String,

    /// 商品数量（必填）
    pub quantity: u32,

    /// 商品单价（必填），单位：元
    pub price: String,

    /// 商品类目（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_category: Option<String>,

    /// 商品类目树（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories_tree: Option<String>,

    /// 商品的展示地址（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_url: Option<String>,
}

/// 业务扩展参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendParams {
    /// 系统商编号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sys_service_provider_id: Option<String>,

    /// 卡类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_type: Option<String>,
}

/// 商户业务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessParams {
    /// 商户端创建订单的 IP，须上传正确的用户端外网 IP，支持 ipv4/ipv6 格式；
    /// mc_create_trade_ip和mcCreateTradeIp（旧）参数描述相同，首选mc_create_trade_ip入参，请勿重复入参；
    /// 如已入参mcCreateTradeIp（旧），无需新增入参mc_create_trade_ip。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mc_create_trade_ip: Option<String>,
}

/// 优惠参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoParam {
    /// 存在延迟扣款这一类的场景，
    /// 用这个时间表明用户发生交易的时间，比如说，在公交地铁场景，用户刷码出站的时间，和商户上送交易的时间是不一样的。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_order_time: Option<String>,
}

/// 查询选项枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryOption {
    /// 资金明细信息
    FundBillList,
    /// 优惠券信息
    VoucherDetailList,
    /// 因公付金额信息
    EnterprisePayInfo,
    /// 惠营宝回票金额信息
    HybAmount,
    /// 商品优惠信息
    DiscountGoodsDetail,
    /// 平台优惠金额
    DiscountAmount,
    /// 商家优惠金额
    MdiscountAmount,
}

impl AlipayTradePayRequest {
    /// 创建当面付基础请求
    pub fn new(
        out_trade_no: impl Into<String>,
        total_amount: impl Into<String>,
        subject: impl Into<String>,
        auth_code: impl Into<String>,
    ) -> Self {
        Self {
            out_trade_no: out_trade_no.into(),
            total_amount: total_amount.into(),
            subject: subject.into(),
            auth_code: auth_code.into(),
            scene: PaymentScene::BarCode,
            product_code: Some(ProductCode::FaceToFacePayment),
            seller_id: None,
            goods_detail: None,
            extend_params: None,
            business_params: None,
            promo_params: None,
            store_id: None,
            operator_id: None,
            terminal_id: None,
            query_options: None,
        }
    }

    /// 设置支付场景
    pub fn with_scene(mut self, scene: PaymentScene) -> Self {
        self.scene = scene;
        self
    }

    /// 设置产品码
    pub fn with_product_code(mut self, product_code: ProductCode) -> Self {
        self.product_code = Some(product_code);
        self
    }

    /// 设置卖家ID
    pub fn with_seller_id(mut self, seller_id: impl Into<String>) -> Self {
        self.seller_id = Some(seller_id.into());
        self
    }

    /// 设置门店编号
    pub fn with_store_id(mut self, store_id: impl Into<String>) -> Self {
        self.store_id = Some(store_id.into());
        self
    }

    /// 设置操作员编号
    pub fn with_operator_id(mut self, operator_id: impl Into<String>) -> Self {
        self.operator_id = Some(operator_id.into());
        self
    }

    /// 设置终端编号
    pub fn with_terminal_id(mut self, terminal_id: impl Into<String>) -> Self {
        self.terminal_id = Some(terminal_id.into());
        self
    }

    /// 添加商品详情
    pub fn with_goods_detail(mut self, goods: Vec<GoodsDetail>) -> Self {
        self.goods_detail = Some(goods);
        self
    }

    /// 添加查询选项
    pub fn with_query_options(mut self, options: Vec<QueryOption>) -> Self {
        self.query_options = Some(options);
        self
    }
}

impl Method for AlipayTradePayRequest {
    fn method() -> &'static str {
        "alipay.trade.pay"
    }
}
