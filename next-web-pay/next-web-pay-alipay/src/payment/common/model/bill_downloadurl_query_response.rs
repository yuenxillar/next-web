use serde::Deserialize;

use crate::Named;

#[derive(Debug, Clone, Deserialize)]
pub struct AlipayTradeBillDownloadurlQueryResponse {
    /// 当账单可获取时，返回账单下载地址链接，获取链接后30秒后未下载，链接地址失效。
    /// 【示例值】http://dwbillcenter.alipay.com/downloadBillFile.resource?bizType=X&pid=X&fileType=X&bizDates=X&downloadFileName=X&fileId=X
    pub bill_download_url: String,

    ///描述本次申请的账单文件状态。 EMPTY_DATA_WITH_BILL_FILE：当天无账单业务数据&&可以获取到空数据账单文件。
    /// 【枚举值】
    /// 空账单数据文件：当前周期无数据时候产生的账单文件: EMPTY_DATA_WITH_BILL_FILE
    /// 【注意事项】目前仅对默认配置用户生效，主动配置的不会返回当前字段。
    /// 【示例值】EMPTY_DATA_WITH_BILL_FILE
    pub bill_file_code: Option<String>,
}

/// 支付宝账单下载查询业务错误码
///
/// 定义了在调用账单下载接口时可能返回的业务级错误类型。
/// 这些错误码对应支付宝网关返回的 `sub_code` 字段，
/// 用于精确识别错误原因并采取相应的处理措施。
///
/// # 错误码来源
/// 参考支付宝开放平台文档：https://opendocs.alipay.com/open/e81ed5f1_alipay.data.dataservice.bill.downloadurl.query?pathHash=52c7a081&scene=common&ref=api
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlipayTradeBillDownloadurlQueryBusinessErrorCode {
    /// BILL_DATE_BEFORE_REGISTRATION
    /// 请求的账单时间早于商户注册时间
    ///
    /// 解决方案：确认账单日期是否正确，确认商户注册时间后重新查询
    BillDateBeforeRegistration,

    /// BILL_NOT_EXIST
    /// 账单不存在
    ///
    /// 解决方案：确认账单日期和账单类型参数是否正确，确认后重新查询
    BillNotExist,

    /// INVAILID_ARGUMENTS
    /// 入参不合法
    ///
    /// 解决方案：检查所有请求参数（日期格式、账单类型等），修正后重新查询
    InvailidArguments,

    /// NO_BILL_DATA
    /// 商户在请求的账单时间内没有发生当前账单类型的业务
    ///
    /// 解决方案：确认所选日期内是否有对应类型的交易发生，确认参数后重新查询
    NoBillData,

    /// SYSTEM_RATE_LIMIT
    /// 系统当前负载较高，请求被限流
    ///
    /// 解决方案：间隔 1 分钟后重试
    SystemRateLimit,

    /// TYPE_NOT_SUPPORTED
    /// 此账单类型不支持下载
    ///
    /// 解决方案：
    /// - bill_type = "trade"：需签约支付宝支付产品，且有实际交易流水
    /// - bill_type = "signcustomer"：非支付宝商家身份，建议先签约收钱码或经营码
    /// - bill_type = "merchant_act"：一年内无营销动作，详见营销账单使用文档
    /// - bill_type = "trade_zft_merchant"：联系直付通平台商提供账单数据
    /// - bill_type = "zft_acc"：需先签约【互联网平台直付通】产品
    /// - bill_type = "settlementMerge"：需先签约【收款到银行账户】产品
    /// - 其他类型：该账单类型不支持下载，确认账单类型和日期，联系支付宝小二排查
    TypeNotSupported,

    /// UNKNOWN_ERROR
    /// 未知错误
    ///
    /// 解决方案：稍后重试，如持续出现请联系支付宝技术支持排查
    UnknownError,

    /// USER_RATE_LIMIT
    /// 调用频率超限
    ///
    /// 解决方案：间隔 1 分钟后重试，注意遵守支付宝的调用频率限制规范
    UserRateLimit,
}

impl std::fmt::Display for AlipayTradeBillDownloadurlQueryBusinessErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BillDateBeforeRegistration => write!(f, "请求的账单时间早于注册时间"),
            Self::BillNotExist => write!(f, "账单不存在"),
            Self::InvailidArguments => write!(f, "入参不合法"),
            Self::NoBillData => write!(f, "商户在请求的账单时间内没有发生当前账单类型的业务"),
            Self::SystemRateLimit => write!(f, "系统当前负载较高，请求被限流"),
            Self::TypeNotSupported => write!(f, "此账单类型不支持下载"),
            Self::UnknownError => write!(f, "未知错误"),
            Self::UserRateLimit => write!(f, "调用频率超限"),
        }
    }
}

impl std::error::Error for AlipayTradeBillDownloadurlQueryBusinessErrorCode {}

impl From<&str> for AlipayTradeBillDownloadurlQueryBusinessErrorCode {
    fn from(code: &str) -> Self {
        match code {
            "BILL_DATE_BEFORE_REGISTRATION" => Self::BillDateBeforeRegistration,
            "BILL_NOT_EXIST" => Self::BillNotExist,
            "INVAILID_ARGUMENTS" => Self::InvailidArguments,
            "NO_BILL_DATA" => Self::NoBillData,
            "SYSTEM_RATE_LIMIT" => Self::SystemRateLimit,
            "TYPE_NOT_SUPPORTED" => Self::TypeNotSupported,
            "UNKNOWN_ERROR" => Self::UnknownError,
            "USER_RATE_LIMIT" => Self::UserRateLimit,
            _ => Self::UnknownError,
        }
    }
}

impl From<String> for AlipayTradeBillDownloadurlQueryBusinessErrorCode {
    fn from(code: String) -> Self {
        Self::from(code.as_str())
    }
}

impl Named for AlipayTradeBillDownloadurlQueryResponse {
    fn name() -> &'static str {
        "alipay_data_dataservice_bill_downloadurl_query_response"
    }
}
