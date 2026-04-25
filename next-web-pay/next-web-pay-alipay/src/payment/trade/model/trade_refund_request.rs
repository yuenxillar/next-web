use serde::Deserialize;

/// 支付宝退款请求参数
#[derive(Debug, Clone, Deserialize)]
pub struct TradeRefundRequest {
    /// 退款金额（必填）
    /// 需要退款的金额，不能大于订单金额，单位为元，支持两位小数
    refund_amount: String,

    // ============ 二选一（至少传一个） ============
    /// 商户订单号
    /// 与 trade_no 不能同时为空
    out_trade_no: Option<String>,

    /// 支付宝交易号
    /// 与 out_trade_no 不能同时为空，两者同时存在时优先取值 trade_no
    trade_no: Option<String>,

    /// 退款原因说明
    pub refund_reason: Option<String>,

    /// 退款请求号
    /// 标识一次退款请求，需保证在交易号下唯一
    /// 部分退款时必传
    pub out_request_no: Option<String>,

    /// 退款包含的商品列表信息
    pub refund_goods_detail: Option<Vec<RefundGoodsDetail>>,

    /// 退分账明细信息
    pub refund_royalty_parameters: Option<Vec<OpenApiRoyaltyDetailInfoPojo>>,

    /// 查询选项
    pub query_options: Option<Vec<RefundQueryOption>>,

    /// 针对账期交易，确认结算后退款时需指定结算单号
    pub related_settle_confirm_no: Option<String>,
}

/// 退款商品详情
#[derive(Debug, Clone, Deserialize)]
pub struct RefundGoodsDetail {
    /// 商品编号（必填）
    pub goods_id: String,

    /// 该商品的退款总金额，单位为元（必填）
    pub refund_amount: String,

    /// 外部商品凭证编号列表
    pub out_certificate_no_list: Option<Vec<String>>,

    /// 商家侧小程序商品ID
    pub out_item_id: Option<String>,

    /// 商家侧小程序商品sku ID
    pub out_sku_id: Option<String>,
}

/// 退分账明细信息
#[derive(Debug, Clone, Deserialize)]
pub struct OpenApiRoyaltyDetailInfoPojo {
    /// 分账类型
    pub royalty_type: Option<RoyaltyType>,

    /// 支出方账户
    pub trans_out: Option<String>,

    /// 支出方账户类型
    pub trans_out_type: Option<TransAccountType>,

    /// 收入方账户类型
    pub trans_in_type: Option<TransInAccountType>,

    /// 收入方账户
    pub trans_in: Option<String>,

    /// 分账的金额，单位为元
    pub amount: Option<String>,

    /// 分账描述
    pub desc: Option<String>,

    /// 分账场景：达人佣金、平台服务费、技术服务费、其他
    pub royalty_scene: Option<String>,

    /// 分账收款方姓名
    pub trans_in_name: Option<String>,
}

/// 分账类型
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoyaltyType {
    /// 分账
    Transfer,
    /// 营销补差
    Replenish,
}

/// 支出方/收入方账户类型
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TransAccountType {
    /// 支付宝账号对应的支付宝唯一用户号
    UserId,
    /// 支付宝登录号
    LoginName,
}

/// 收入方账户类型（比支出方多一个卡编号）
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TransInAccountType {
    /// 支付宝账号对应的支付宝唯一用户号
    UserId,
    /// 支付宝登录号
    LoginName,
    /// 卡编号
    CardAliasNo,
}

/// 退款查询选项
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RefundQueryOption {
    /// 本次退款使用的资金渠道
    RefundDetailItemList,
    /// 银行卡冲退信息
    DepositBackInfo,
    /// 本次退款退的券信息
    RefundVoucherDetailList,
}

impl TradeRefundRequest {

    pub fn new(
        out_trade_no: impl Into<Option<String>>,
        trade_no: impl Into<Option<String>>,
        refund_amount: impl Into<String>,
    ) -> Self {
        let out_trade_no = out_trade_no.into();
        let trade_no = trade_no.into();
        let refund_amount = refund_amount.into();

        Self {
            out_trade_no,
            trade_no,
            refund_amount,
            refund_reason: None,
            out_request_no: None,
            refund_goods_detail: None,
            refund_royalty_parameters: None,
            query_options: None,
            related_settle_confirm_no: None,
        }
    }

    /// 设置商户订单号
    pub fn with_out_trade_no(mut self, out_trade_no: impl Into<String>) -> Self {
        self.out_trade_no = Some(out_trade_no.into());
        self
    }

    /// 设置支付宝交易号
    pub fn with_trade_no(mut self, trade_no: impl Into<String>) -> Self {
        self.trade_no = Some(trade_no.into());
        self
    }

    /// 设置退款原因
    pub fn with_refund_reason(mut self, reason: impl Into<String>) -> Self {
        self.refund_reason = Some(reason.into());
        self
    }

    /// 设置退款请求号 (部分退款时必传)
    pub fn with_out_request_no(mut self, request_no: impl Into<String>) -> Self {
        self.out_request_no = Some(request_no.into());
        self
    }

    /// 设置退款商品明细
    pub fn with_refund_goods_detail(mut self, goods: Vec<RefundGoodsDetail>) -> Self {
        self.refund_goods_detail = Some(goods);
        self
    }

    /// 设置分账明细
    pub fn with_refund_royalty_parameters(
        mut self,
        params: Vec<OpenApiRoyaltyDetailInfoPojo>,
    ) -> Self {
        self.refund_royalty_parameters = Some(params);
        self
    }

    /// 设置查询选项
    pub fn with_query_options(mut self, options: Vec<RefundQueryOption>) -> Self {
        self.query_options = Some(options);
        self
    }

    /// 设置关联结算单号
    pub fn with_related_settle_confirm_no(mut self, confirm_no: impl Into<String>) -> Self {
        self.related_settle_confirm_no = Some(confirm_no.into());
        self
    }

    /// 确保 out_trade_no 和 trade_no 至少有一个存在
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.out_trade_no.is_none() && self.trade_no.is_none() {
            return Err("out_trade_no and trade_no cannot both be empty");
        }
        Ok(())
    }
}
