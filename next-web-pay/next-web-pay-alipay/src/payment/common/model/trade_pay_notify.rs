use serde::{Deserialize, Deserializer};

/// 支付宝交易状态同步通知
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AlipayTradePayNotify {
    /// 通知时间，格式 yyyy-MM-dd HH:mm:ss
    pub notify_time: String,

    /// 通知类型，枚举值：trade_status_sync
    pub notify_type: String,

    /// 通知校验ID
    pub notify_id: String,

    /// 签名类型，如 RSA2
    pub sign_type: String,

    /// 签名
    pub sign: String,

    /// 支付宝交易号
    pub trade_no: String,

    /// 开发者的app_id
    pub app_id: String,

    /// 开发者的 app_id，在服务商调用的场景下为授权方的 app_id
    pub auth_app_id: String,

    /// 商户订单号
    pub out_trade_no: String,

    /// 商家业务号。商家业务 ID，主要是退款通知中返回退款申请的流水号
    pub out_biz_no: Option<String>,

    /// 买家支付宝用户号（旧字段，新商户建议使用buyer_open_id）
    pub buyer_id: Option<String>,

    /// 买家支付宝账号
    pub buyer_open_id: Option<String>,

    /// 买家支付宝账号
    pub buyer_logon_id: Option<String>,

    /// 卖家支付宝用户号
    pub seller_id: Option<String>,

    /// 卖家支付宝账号
    pub seller_email: Option<String>,

    /// 交易状态，如 TRADE_CLOSED
    pub trade_status: TradeStatus,

    /// 订单金额（元），支持两位小数
    pub total_amount: String,

    /// 实收金额（元），支持两位小数
    pub receipt_amount: String,

    /// 开票金额（元）
    pub invoice_amount: Option<String>,

    /// 付款金额（元）
    pub buyer_pay_amount: Option<String>,

    /// 集分宝金额（元）
    pub point_amount: Option<String>,

    /// 总退款金额（元）
    pub refund_fee: Option<String>,

    /// 实际退款金额（元）
    pub send_back_fee: Option<String>,

    /// 订单标题
    pub subject: Option<String>,

    /// 商品描述
    pub body: Option<String>,

    /// 交易创建时间，格式 yyyy-MM-dd HH:mm:ss
    pub gmt_create: Option<String>,

    /// 交易付款时间
    pub gmt_payment: Option<String>,

    /// 交易退款时间，格式 yyyy-MM-dd HH:mm:ss.SS
    pub gmt_refund: Option<String>,

    /// 交易结束时间
    pub gmt_close: Option<String>,

    /// 支付金额信息（JSON数组字符串）
    #[serde(default, deserialize_with = "deserialize_fund_bill_list")]
    pub fund_bill_list: Option<Vec<FundBill>>,
    // pub fund_bill_list: Option<String>,
    /// 优惠券信息（JSON数组字符串）
    #[serde(default, deserialize_with = "deserialize_voucher_detail_list")]
    pub voucher_detail_list: Option<Vec<VoucherDetail>>,

    /// 账期结算标识（如 PERIOD）
    pub biz_settle_mode: Option<String>,
}

impl AlipayTradePayNotify {
    /// 判断交易是否成功（包含成功和结束状态）
    pub fn is_success(&self) -> bool {
        matches!(
            self.trade_status,
            TradeStatus::TradeSuccess | TradeStatus::TradeFinished
        )
    }

    /// 判断交易是否关闭（未付款超时或全额退款）
    pub fn is_closed(&self) -> bool {
        matches!(self.trade_status, TradeStatus::TradeClosed)
    }
}

fn deserialize_fund_bill_list<'de, D>(deserializer: D) -> Result<Option<Vec<FundBill>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;

    match s {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => serde_json::from_str(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

fn deserialize_voucher_detail_list<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<VoucherDetail>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;

    println!("deserialize_voucher_detail_list: {:?}\n\n", s);

    match s {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => serde_json::from_str(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

/// 交易状态
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
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

/// 资金明细
#[derive(Debug, Clone, Deserialize)]
pub struct FundBill {
    /// 金额
    pub amount: String,
    /// 资金渠道，如 ALIPAYACCOUNT
    #[serde(rename = "fundChannel")]
    pub fund_channel: FundChannel,
}

/// 支付宝资金渠道/支付方式
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum FundChannel {
    /// 支付宝红包
    #[serde(rename = "COUPON")]
    Coupon,

    /// 支付宝账户余额
    #[serde(rename = "ALIPAYACCOUNT")]
    AlipayAccount,

    /// 集分宝
    #[serde(rename = "POINT")]
    Point,

    /// 折扣券
    #[serde(rename = "DISCOUNT")]
    Discount,

    /// 预付卡
    #[serde(rename = "PCARD")]
    Pcard,

    /// 商家储值卡
    #[serde(rename = "MCARD")]
    Mcard,

    /// 商户优惠券
    #[serde(rename = "MDISCOUNT")]
    MDiscount,

    /// 商户红包
    #[serde(rename = "MCOUPON")]
    MCoupon,

    /// 银行卡
    #[serde(rename = "BANKCARD")]
    BankCard,

    /// 余额宝
    #[serde(rename = "MONEYFUND")]
    MoneyFund,

    /// 券
    #[serde(rename = "VOUCHER")]
    Voucher,

    /// 数字人民币
    #[serde(rename = "DCEP_ASSET")]
    DcepAsset,

    /// 未知的资金渠道（预留扩展）
    #[serde(other)]
    Unknown,
}

/// 优惠券信息
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherDetail {
    /// 券 ID
    pub voucher_id: String,

    /// 券模板 ID
    pub template_id: Option<String>,

    /// 券名称
    pub name: String,

    /// 券类型，如 ALIPAY_BIZ_VOUCHER
    #[serde(rename = "type")]
    pub voucher_type: String,

    /// 金额
    pub amount: String,

    /// 商家出资
    pub merchant_contribute: Option<String>,

    /// 其他出资
    pub other_contribute: Option<String>,

    /// 优惠券的其他出资方明细
    pub other_contribute_detail: Option<Vec<String>>,

    /// 	出资方类型，如品牌商出资、支付宝平台出资等
    #[serde(rename = "L contributeType")]
    pub l_contribute_type: Option<String>,

    /// 出资方金额
    #[serde(rename = "L contributeAmount")]
    pub l_contribute_amount: Option<String>,

    /// 备注
    pub memo: Option<String>,
}
