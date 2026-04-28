use serde::Deserialize;

use crate::Named;

/// 支付宝交易支付响应数据
#[derive(Debug, Clone, Deserialize)]
pub struct AlipayTradePayResponse {
    /// 商户订单号（必填）
    /// 64个字符以内
    pub out_trade_no: String,

    /// 交易金额（必填）
    /// 单位：元，精确到小数点后两位
    pub total_amount: String,

    /// 实收金额（必填）
    /// 单位：元
    pub receipt_amount: String,

    /// 交易支付时间（必填）
    /// 格式：2014-11-27 15:45:57
    pub gmt_payment: String,

    /// 交易支付使用的资金渠道（必填）
    pub fund_bill_list: Vec<TradeFundBill>,

    /// 买家在支付宝的用户id（可选）
    /// 28个字符
    /// 建议新商户使用 buyer_open_id
    pub buyer_user_id: Option<String>,

    /// 买家支付宝用户唯一标识（可选）
    /// 128个字符
    pub buyer_open_id: Option<String>,

    /// 平台优惠金额（特殊可选）
    pub discount_amount: Option<String>,

    /// 商家优惠金额（特殊可选）
    pub mdiscount_amount: Option<String>,

    /// 支付宝交易号（可选）
    /// 64个字符
    /// 未生成真实交易时不返回
    pub trade_no: Option<String>,

    /// 买家支付宝账号（可选）
    /// 100个字符
    pub buyer_logon_id: Option<String>,

    /// 买家付款的金额（可选）
    /// 单位：元
    pub buyer_pay_amount: Option<String>,

    /// 使用集分宝付款的金额（可选）
    /// 单位：元
    pub point_amount: Option<String>,

    /// 交易中可给用户开具发票的金额（可选）
    /// 单位：元
    pub invoice_amount: Option<String>,

    /// 发生支付交易的商户门店名称（可选）
    /// 512个字符
    pub store_name: Option<String>,

    /// 单品券优惠的商品优惠信息（可选）
    /// 5120个字符
    /// 需在 query_options 中指定才返回
    pub discount_goods_detail: Option<String>,

    /// 支付时使用的所有优惠券信息（可选）
    /// 需在 query_options 中指定才返回
    pub voucher_detail_list: Option<Vec<VoucherDetail>>,
}

/// 交易支付使用的资金渠道
#[derive(Debug, Clone, Deserialize)]
pub struct TradeFundBill {
    /// 交易使用的资金渠道（必填）
    /// 32个字符
    pub fund_channel: String,

    /// 该支付工具类型所使用的金额（必填）
    /// 单位：元
    pub amount: String,

    /// 渠道实际付款金额（可选）
    pub real_amount: Option<String>,
}

/// 支付时使用的优惠券信息
#[derive(Debug, Clone, Deserialize)]
pub struct VoucherDetail {
    /// 券id（必填）
    /// 32个字符
    pub id: String,

    /// 券名称（必填）
    /// 64个字符
    pub name: String,

    /// 券类型（必填）
    /// 32个字符
    /// ALIPAY_FIX_VOUCHER - 全场代金券
    /// ALIPAY_DISCOUNT_VOUCHER - 折扣券
    /// ALIPAY_ITEM_VOUCHER - 单品优惠券
    /// ALIPAY_CASH_VOUCHER - 现金抵价券
    /// ALIPAY_BIZ_VOUCHER - 商家全场券
    #[serde(rename = "type")]
    pub voucher_type: String,

    /// 优惠券面额（必填）
    /// 单位：元
    pub amount: String,

    /// 商家出资金额（可选）
    pub merchant_contribute: Option<String>,

    /// 其他出资方出资金额（可选）
    pub other_contribute: Option<String>,

    /// 优惠券备注信息（可选）
    /// 256个字符
    pub memo: Option<String>,

    /// 券模板id（可选）
    /// 64个字符
    pub template_id: Option<String>,

    /// 用户购买券时实际付款金额（可选）
    pub purchase_buyer_contribute: Option<String>,

    /// 用户购买券时商户优惠金额（可选）
    pub purchase_merchant_contribute: Option<String>,

    /// 用户购买券时平台优惠金额（可选）
    pub purchase_ant_contribute: Option<String>,
}

impl AlipayTradePayResponse {
    /// 获取买家标识（优先使用 open_id）
    pub fn buyer_id(&self) -> Option<&str> {
        self.buyer_open_id
            .as_deref()
            .or(self.buyer_user_id.as_deref())
    }

    /// 是否已生成真实交易
    pub fn has_trade(&self) -> bool {
        self.trade_no.is_some()
    }

    /// 获取实际支付金额（优先实收金额）
    pub fn paid_amount(&self) -> &str {
        self.buyer_pay_amount
            .as_deref()
            .unwrap_or(&self.total_amount)
    }
}

impl Named for AlipayTradePayResponse {
    fn name() -> &'static str {
        "alipay_trade_pay_response"
    }
}
