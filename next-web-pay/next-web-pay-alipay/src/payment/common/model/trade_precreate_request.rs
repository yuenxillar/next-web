use serde::Serialize;

use crate::Method;

/// 统一收单交易支付预创建
#[derive(Debug, Serialize)]
pub struct AlipayTradePrecreateRequest {
    /// 商户订单号。必选，64个字符以内，仅支持字母、数字、下划线
    pub out_trade_no: String,

    /// 订单总金额。必选，单位为元，精确到小数点后两位
    pub total_amount: String,

    /// 订单标题。必选，256个字符以内，不可使用特殊字符如 /, =, & 等
    pub subject: String,

    /// 产品码。必选，订单码支付传：QR_CODE_OFFLINE
    pub product_code: String,

    /// 卖家支付宝用户ID。可选
    pub seller_id: Option<String>,

    /// 订单附加信息。可选
    pub body: Option<String>,

    /// 订单包含的商品列表信息
    pub goods_detail: Option<Vec<GoodsDetail>>,

    /// 业务扩展参数
    pub extend_params: Option<ExtendParams>,

    /// 商户传入业务信息
    pub business_params: Option<BusinessParams>,

    /// 可打折金额
    pub discountable_amount: Option<String>,

    /// 商户门店编号
    pub store_id: Option<String>,

    /// 商户操作员编号
    pub operator_id: Option<String>,

    /// 商户机具终端编号
    pub terminal_id: Option<String>,

    /// 商户的原始订单号
    pub merchant_order_no: Option<String>,
}

// ==================== 商品明细 ====================

/// 商品明细
#[derive(Debug, Serialize, Clone)]
pub struct GoodsDetail {
    /// 商品的编号。必选
    pub goods_id: String,

    /// 商品名称。必选
    pub goods_name: String,

    /// 商品数量。必选
    pub quantity: u32,

    /// 商品单价，单位为元。必选
    pub price: String,

    /// 商品类目。可选
    pub goods_category: Option<String>,

    /// 商品类目树。可选
    pub categories_tree: Option<String>,

    /// 商品的展示地址。可选
    pub show_url: Option<String>,
}

/// 业务扩展参数
#[derive(Debug, Serialize, Default, Clone)]
pub struct ExtendParams {
    /// 系统商编号
    pub sys_service_provider_id: Option<String>,
}

/// 商户传入业务信息
#[derive(Debug, Serialize, Default, Clone)]
pub struct BusinessParams {
    /// 具体业务信息，格式为 json
    pub data: Option<String>,

    /// 商户端创建订单的 IP，须上传正确的用户端外网 IP，支持 ipv4/ipv6 格式
    pub mc_create_trade_ip: Option<String>,
}

impl AlipayTradePrecreateRequest {
    pub fn new(
        out_trade_no: impl Into<String>,
        total_amount: impl Into<String>,
        subject: impl Into<String>,
        product_code: Option<String>,
    ) -> Self {
        Self {
            out_trade_no: out_trade_no.into(),
            total_amount: total_amount.into(),
            subject: subject.into(),
            product_code: product_code.unwrap_or("FACE_TO_FACE_PAYMENT".into()),
            seller_id: None,
            body: None,
            goods_detail: None,
            extend_params: None,
            business_params: None,
            discountable_amount: None,
            store_id: None,
            operator_id: None,
            terminal_id: None,
            merchant_order_no: None,
        }
    }

    /// 设置卖家支付宝用户ID
    pub fn with_seller_id(mut self, seller_id: String) -> Self {
        self.seller_id = Some(seller_id);
        self
    }

    /// 设置订单附加信息
    pub fn with_body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    /// 设置商品列表
    pub fn with_goods_detail(mut self, goods_detail: Vec<GoodsDetail>) -> Self {
        self.goods_detail = Some(goods_detail);
        self
    }

    /// 设置业务扩展参数
    pub fn with_extend_params(mut self, extend_params: ExtendParams) -> Self {
        self.extend_params = Some(extend_params);
        self
    }

    /// 设置业务参数
    pub fn with_business_params(mut self, business_params: BusinessParams) -> Self {
        self.business_params = Some(business_params);
        self
    }

    /// 设置可打折金额
    pub fn with_discountable_amount(mut self, amount: String) -> Self {
        self.discountable_amount = Some(amount);
        self
    }

    /// 设置门店编号
    pub fn with_store_id(mut self, store_id: String) -> Self {
        self.store_id = Some(store_id);
        self
    }

    /// 设置操作员编号
    pub fn with_operator_id(mut self, operator_id: String) -> Self {
        self.operator_id = Some(operator_id);
        self
    }

    /// 设置终端编号
    pub fn with_terminal_id(mut self, terminal_id: String) -> Self {
        self.terminal_id = Some(terminal_id);
        self
    }

    /// 设置商户原始订单号
    pub fn with_merchant_order_no(mut self, merchant_order_no: String) -> Self {
        self.merchant_order_no = Some(merchant_order_no);
        self
    }
}

impl GoodsDetail {
    pub fn new(goods_id: String, goods_name: String, quantity: u32, price: String) -> Self {
        Self {
            goods_id,
            goods_name,
            quantity,
            price,
            goods_category: None,
            categories_tree: None,
            show_url: None,
        }
    }

    pub fn with_category(mut self, category: String) -> Self {
        self.goods_category = Some(category);
        self
    }

    pub fn with_categories_tree(mut self, tree: String) -> Self {
        self.categories_tree = Some(tree);
        self
    }

    pub fn with_show_url(mut self, url: String) -> Self {
        self.show_url = Some(url);
        self
    }
}

impl Method for AlipayTradePrecreateRequest {
    fn method() -> &'static str {
        "alipay.trade.precreate"
    }
}
