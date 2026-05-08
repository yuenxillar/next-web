use serde::Serialize;

use crate::Method;

#[derive(Debug, Clone, Serialize)]
pub struct AlipayTradeBillDownloadurlQueryRequest {
    /// 账单类型，商户通过接口或商户经开放平台授权后其所属服务商通过接口可以获取以下账单类型
    /// 【示例值】trade
    bill_type: String,

    /// 账单时间：
    /// 日账单格式为yyyy-MM-dd，最早可下载近6年的日账单。不支持下载当日账单，只能下载前一日24点前的账单数据（T+1），当日数据一般于次日 9 点前生成，特殊情况可能延迟
    /// 月账单格式为yyyy-MM，最早可下载近6年的月账单。不支持下载当月账单，只能下载上一月账单数据，当月账单一般在次月 3 日生成，特殊情况可能延迟
    /// 当biz_type为settlementMerge时候，时间为汇总批次结算资金到账的日期，日期格式为yyyy-MM-dd，最早可下载2023年4月17日及以后的账单
    /// 【示例值】2025-05-01
    bill_date: String,

    /// 二级商户smid，这个参数只在bill_type是trade_zft_merchant时才能使用
    /// 【示例值】2088123412341234
    #[serde(skip_serializing_if = "Option::is_none")]
    smid: Option<String>,
}

impl AlipayTradeBillDownloadurlQueryRequest {
    pub fn new(bill_type: BillType, bill_date: impl Into<String>) -> Self {
        Self {
            bill_type: bill_type.to_string(),
            bill_date: bill_date.into(),
            smid: None,
        }
    }

    pub fn with_smid(mut self, smid: impl Into<String>) -> Self {
        self.smid = Some(smid.into());

        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillType {
    Trade,
    Signcustomer,
    MerchantAct,
    TradeZftMerchant,
    ZftAcc,
    SettlementMerge,
    Custom(String),
}

impl BillType {
    fn to_string(self) -> String {
        match self {
            BillType::Trade => "trade".into(),
            BillType::Signcustomer => "signcustomer".into(),
            BillType::MerchantAct => "merchant_act".into(),
            BillType::TradeZftMerchant => "trade_zft_merchant".into(),
            BillType::ZftAcc => "zft_acc".into(),
            BillType::SettlementMerge => "settlementMerge".into(),
            BillType::Custom(ty) => ty,
        }
    }
}

impl Method for AlipayTradeBillDownloadurlQueryRequest {
    fn method() -> &'static str {
        "alipay.data.dataservice.bill.downloadurl.query"
    }
}
