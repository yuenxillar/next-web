use serde::Serialize;

use crate::Path;

/// WeChat Pay V3 Global MicroPay request.
///
/// Both direct-connection and institution modes are supported.
/// Use the fields relevant to your merchant type.
#[derive(Debug, Clone, Default, Serialize)]
pub struct WechatPayMicroPayRequest {
    /// 商户号
    mchid: String,

    /// app ID.
    appid: String,

    /// 子商户号
    sub_mchid: String,

    /// 机构商户号
    sp_mchid: String,

    /// 机构APPID
    sp_appid: String,

    /// 子商户APPID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_appid: Option<String>,

    /// 商品描述
    description: String,

    /// 商户数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<String>,

    /// 商户订单号
    out_trade_no: String,

    /// 商品标记
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_tag: Option<String>,

    /// 交易类型
    trade_type: String,

    /// MCC码
    merchant_category_code: String,

    /// 支付者信息
    payer: MicroPayPayer,

    /// Order amount.
    amount: MicroPayAmount,

    /// 场景信息对象
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_info: Option<MicroPaySceneInfo>,

    /// 交易购买商品或服务详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<Vec<MicroPayDetail>>,
}

impl WechatPayMicroPayRequest {
    pub fn new(
        appid: impl Into<String>,
        mchid: impl Into<String>,
        sub_mchid: impl Into<String>,
        sp_mchid: impl Into<String>,
        sp_appid: impl Into<String>,
        description: impl Into<String>,
        out_trade_no: impl Into<String>,
        trade_type: impl Into<String>,
        merchant_category_code: impl Into<String>,
        payer: MicroPayPayer,
        amount: MicroPayAmount,
    ) -> Self {
        Self {
            appid: appid.into(),
            mchid: mchid.into(),
            sub_mchid: sub_mchid.into(),
            sp_mchid: sp_mchid.into(),
            sp_appid: sp_appid.into(),
            description: description.into(),
            out_trade_no: out_trade_no.into(),
            trade_type: trade_type.into(),
            merchant_category_code: merchant_category_code.into(),
            payer,
            amount,
            ..Default::default()
        }
    }

    pub fn with_sub_appid(mut self, sub_appid: impl Into<String>) -> Self {
        self.sub_appid = Some(sub_appid.into());
        self
    }

    pub fn with_attach(mut self, attach: impl Into<String>) -> Self {
        self.attach = Some(attach.into());
        self
    }

    pub fn with_goods_tag(mut self, goods_tag: impl Into<String>) -> Self {
        self.goods_tag = Some(goods_tag.into());
        self
    }

    pub fn with_scene_info(mut self, scene_info: MicroPaySceneInfo) -> Self {
        self.scene_info = Some(scene_info);
        self
    }

    pub fn with_detail(mut self, detail: Vec<MicroPayDetail>) -> Self {
        self.detail = Some(detail);
        self
    }
}

/// Payer information for micropay.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MicroPayPayer {
    /// 扫码支付授权码，即用户打开微信钱包显示的码
    pub auth_code: String,
}

/// Order amount for micropay.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MicroPayAmount {
    /// 总金额
    ///
    /// 订单总金额，币种的最小单位，只能为整数，详见交易金额
    pub total: u32,

    /// 货币类型
    ///
    /// 符合ISO 4217标准的三位字母代码，默认仅支持使用mch_id 对应的结算币种或 CNY，如结算币种为 USD，
    /// 则currency 可使用 USD 或 CNY，如需使用非结算币种标价，请联系微信支付的区域运营申请
    pub currency: String,
}

/// Scene info for micropay.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MicroPaySceneInfo {
    /// 商户端设备号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,

    /// 商户端设备IP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_ip: Option<String>,

    /// 用户终端IP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_client_ip: Option<String>,

    /// 操作员ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<String>,

    /// 商户门店信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_info: Option<MicroPayStoreInfo>,
}

/// Store info for scene info.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MicroPayStoreInfo {
    /// 编号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// 名称
    pub name: String,

    /// 详细地址
    pub address: String,
}

/// Detail item in the detail array.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MicroPayDetail {
    /// 商品列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub goods_detail: Vec<MicroPayGoodsDetail>,

    /// Original order price for anti-fraud.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_price: Option<u32>,

    /// Merchant receipt ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
}

/// Goods detail for the detail array.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MicroPayGoodsDetail {
    /// 商品编码
    pub goods_id: String,

    /// 微信支付商品编码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wxpay_goods_id: Option<String>,

    /// 商品名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goods_name: Option<String>,

    /// 商品数量
    pub quantity: u32,

    /// 商品种类
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<u64>,

    /// Unit price in the smallest currency unit.
    pub price: u32,
}

impl Path for WechatPayMicroPayRequest {
    fn path() -> &'static str {
        "/v3/global/micropay/transactions/pay"
    }
}
