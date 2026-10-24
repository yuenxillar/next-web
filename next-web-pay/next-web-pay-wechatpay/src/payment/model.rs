use std::fmt::Debug;

use serde::{Deserialize, Deserializer, de::DeserializeOwned};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum WechatPayResponse<T>
where
    T: serde::de::DeserializeOwned,
{
    Success(T),
    Error(ErrorResponse),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ErrorResponse {
    /// 错误码
    pub code: String,

    /// 错误描述
    pub message: String,

    /// 错误详情
    pub detail: ErrorDetail,
}

/// 错误详情
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ErrorDetail {
    /// 字段
    pub field: String,

    /// 错误值
    pub value: String,

    /// 错误问题
    pub issue: String,

    /// 字段位置
    pub location: String,
}

impl<T> WechatPayResponse<T>
where
    T: serde::de::DeserializeOwned,
{
    pub fn data(self) -> T {
        match self {
            Self::Success(data) => data,
            Self::Error(_) => panic!("ErrorResponse does not have data"),
        }
    }

    // pub fn to_error_string(self) -> String {
    //     match self.data {
    //         Response::Success(_) => panic!("SuccessResponse does not have error"),
    //         Response::Error(error) => format!(
    //             "code: {}, msg: {}, sub_code: {}, sub_msg: {}",
    //             self.code, self.msg, error.sub_code, error.sub_msg
    //         ),
    //     }
    // }
}

impl<'de, T> Deserialize<'de> for WechatPayResponse<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // 1. 解析整个响应为 Value
        let value: Value = Deserialize::deserialize(deserializer)?;

        // 2. 检查是否为错误响应
        let is_err_resp = value.get("code").is_some()
            && value.get("message").is_some()
            && value.get("detail").is_some();

        // 3. 提前返回错误响应
        if is_err_resp {
            let error: ErrorResponse = serde_json::from_value(value).map_err(Error::custom)?;

            return Ok(WechatPayResponse::Error(error));
        }

        // 4. 反序列化业务数据 <T>
        let data: T = serde_json::from_value(value).map_err(Error::custom)?;

        Ok(WechatPayResponse::Success(data))
    }
}

pub mod common {

    use serde::{Deserialize, Serialize};

    /// 交易状态
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "UPPERCASE")]
    pub enum TradeState {
        /// 支付成功
        Success,
        /// 转入退款
        Refund,
        /// 未支付
        Notpay,
        /// 已关闭
        Closed,
        /// 已撤销（刷卡支付）
        Revoked,
        /// 用户支付中
        Userpaying,
        /// 支付失败
        Payerror,
    }

    /// 货币类型（ISO 4217）
    #[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "UPPERCASE")]
    pub enum Currency {
        /// 人民币
        #[default]
        Cny,

        /// 港币
        Hkd,

        /// 英镑
        Gbp,

        /// 美元
        Usd,

        /// 日元
        Jpy,

        /// 加拿大元
        Cad,

        /// 澳大利亚元
        Aud,

        /// 欧元
        Eur,

        /// 新西兰元
        Nzd,

        /// 韩元
        Krw,

        /// 泰铢
        Thb,

        /// 新加坡元
        Sgd,

        /// 卢布
        Rub,

        /// 瑞士法郎
        Chf,

        /// 瑞典克朗
        Sek,

        /// 丹麦克朗
        Dkk,

        /// 挪威克朗
        Nok,

        /// 阿联酋迪拉姆
        Aed,

        /// 土耳其里拉
        Try,

        /// 澳门元
        Mop,

        /// 新台币
        Twd,

        /// 捷克克朗
        Czk,

        /// 马来西亚林吉特
        Myr,

        /// 菲律宾比索
        Php,

        /// 印尼盾
        Idr,

        /// 蒙古图格里克
        Mnt,

        /// 卡塔尔里亚尔
        Qar,

        /// 匈牙利福林
        Huf,

        /// 柬埔寨瑞尔
        Khr,

        /// 冰岛克朗
        Isk,
    }

    /// 微信支付银行类型
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum BankType {
        // ============ 大型国有银行 ============
        /// 工商银行(借记卡)
        #[serde(rename = "ICBC_DEBIT")]
        IcbcDebit,
        /// 工商银行(信用卡)
        #[serde(rename = "ICBC_CREDIT")]
        IcbcCredit,
        /// 农业银行(借记卡)
        #[serde(rename = "ABC_DEBIT")]
        AbcDebit,
        /// 农业银行(信用卡)
        #[serde(rename = "ABC_CREDIT")]
        AbcCredit,
        /// 中国银行(借记卡)
        #[serde(rename = "BOC_DEBIT")]
        BocDebit,
        /// 中国银行(信用卡)
        #[serde(rename = "BOC_CREDIT")]
        BocCredit,
        /// 建设银行(借记卡)
        #[serde(rename = "CCB_DEBIT")]
        CcbDebit,
        /// 建设银行(信用卡)
        #[serde(rename = "CCB_CREDIT")]
        CcbCredit,
        /// 交通银行(借记卡)
        #[serde(rename = "COMM_DEBIT")]
        CommDebit,
        /// 交通银行(信用卡)
        #[serde(rename = "COMM_CREDIT")]
        CommCredit,
        /// 邮政储蓄银行(借记卡)
        #[serde(rename = "PSBC_DEBIT")]
        PsbcDebit,
        /// 邮政储蓄银行(信用卡)
        #[serde(rename = "PSBC_CREDIT")]
        PsbcCredit,

        // ============ 股份制商业银行 ============
        /// 招商银行(借记卡)
        #[serde(rename = "CMB_DEBIT")]
        CmbDebit,
        /// 招商银行(信用卡)
        #[serde(rename = "CMB_CREDIT")]
        CmbCredit,
        /// 浦发银行(借记卡)
        #[serde(rename = "SPDB_DEBIT")]
        SpdbDebit,
        /// 浦发银行(信用卡)
        #[serde(rename = "SPDB_CREDIT")]
        SpdbCredit,
        /// 广发银行(借记卡)
        #[serde(rename = "GDB_DEBIT")]
        GdbDebit,
        /// 广发银行(信用卡)
        #[serde(rename = "GDB_CREDIT")]
        GdbCredit,
        /// 民生银行(借记卡)
        #[serde(rename = "CMBC_DEBIT")]
        CmbcDebit,
        /// 民生银行(信用卡)
        #[serde(rename = "CMBC_CREDIT")]
        CmbcCredit,
        /// 平安银行(借记卡)
        #[serde(rename = "PAB_DEBIT")]
        PabDebit,
        /// 平安银行(信用卡)
        #[serde(rename = "PAB_CREDIT")]
        PabCredit,
        /// 光大银行(借记卡)
        #[serde(rename = "CEB_DEBIT")]
        CebDebit,
        /// 光大银行(信用卡)
        #[serde(rename = "CEB_CREDIT")]
        CebCredit,
        /// 兴业银行(借记卡)
        #[serde(rename = "CIB_DEBIT")]
        CibDebit,
        /// 兴业银行(信用卡)
        #[serde(rename = "CIB_CREDIT")]
        CibCredit,
        /// 中信银行(借记卡)
        #[serde(rename = "CITIC_DEBIT")]
        CiticDebit,
        /// 中信银行(信用卡)
        #[serde(rename = "CITIC_CREDIT")]
        CiticCredit,
        /// 华夏银行(借记卡)
        #[serde(rename = "HXB_DEBIT")]
        HxbDebit,
        /// 华夏银行(信用卡)
        #[serde(rename = "HXB_CREDIT")]
        HxbCredit,

        // ============ 城市商业银行 ============
        /// 上海银行(借记卡)
        #[serde(rename = "BOSH_DEBIT")]
        BoshDebit,
        /// 上海银行(信用卡)
        #[serde(rename = "BOSH_CREDIT")]
        BoshCredit,
        /// 北京银行(借记卡)
        #[serde(rename = "BOB_DEBIT")]
        BobDebit,
        /// 北京银行(信用卡)
        #[serde(rename = "BOB_CREDIT")]
        BobCredit,
        /// 宁波银行(借记卡)
        #[serde(rename = "NBCB_DEBIT")]
        NbcbDebit,
        /// 宁波银行(信用卡)
        #[serde(rename = "NBCB_CREDIT")]
        NbcbCredit,
        /// 江苏银行(借记卡)
        #[serde(rename = "JSB_DEBIT")]
        JsbDebit,
        /// 江苏银行(信用卡)
        #[serde(rename = "JSB_CREDIT")]
        JsbCredit,
        /// 南京银行(借记卡)
        #[serde(rename = "NJCB_DEBIT")]
        NjcbDebit,
        /// 杭州银行(借记卡)
        #[serde(rename = "HZB_DEBIT")]
        HzbDebit,
        /// 杭州银行(信用卡)
        #[serde(rename = "HZB_CREDIT")]
        HzbCredit,
        /// 重庆银行(借记卡)
        #[serde(rename = "CQB_DEBIT")]
        CqbDebit,
        /// 徽商银行(借记卡)
        #[serde(rename = "HSB_DEBIT")]
        HsbDebit,
        /// 徽商银行(信用卡)
        #[serde(rename = "HSB_CREDIT")]
        HsbCredit,
        /// 渤海银行(借记卡)
        #[serde(rename = "CBHB_DEBIT")]
        CbhbDebit,
        /// 哈尔滨银行(借记卡)
        #[serde(rename = "HRBB_DEBIT")]
        HrbbDebit,
        /// 包商银行(借记卡)
        #[serde(rename = "BSB_DEBIT")]
        BsbDebit,
        /// 包商银行(信用卡)
        #[serde(rename = "BSB_CREDIT")]
        BsbCredit,
        /// 盛京银行(借记卡)
        #[serde(rename = "SJB_DEBIT")]
        SjbDebit,
        /// 大连银行(借记卡)
        #[serde(rename = "DLB_DEBIT")]
        DlbDebit,
        /// 大连银行(信用卡)
        #[serde(rename = "DLB_CREDIT")]
        DlbCredit,
        /// 长沙银行(借记卡)
        #[serde(rename = "CSCB_DEBIT")]
        CscbDebit,
        /// 广州银行(借记卡)
        #[serde(rename = "GZCB_DEBIT")]
        GzcbDebit,
        /// 广州银行(信用卡)
        #[serde(rename = "GZCB_CREDIT")]
        GzcbCredit,
        /// 成都银行(借记卡)
        #[serde(rename = "BOCD_DEBIT")]
        BocdDebit,
        /// 河北银行(借记卡)
        #[serde(rename = "BHB_DEBIT")]
        BhbDebit,
        /// 天津银行(借记卡)
        #[serde(rename = "TJB_DEBIT")]
        TjbDebit,
        /// 郑州银行(借记卡)
        #[serde(rename = "ZZB_DEBIT")]
        ZzbDebit,
        /// 郑州银行(信用卡)
        #[serde(rename = "ZZB_CREDIT")]
        ZzbCredit,
        /// 贵阳银行(借记卡)
        #[serde(rename = "GYCB_DEBIT")]
        GycbDebit,
        /// 贵阳银行(信用卡)
        #[serde(rename = "GYCB_CREDIT")]
        GycbCredit,
        /// 西安银行(借记卡)
        #[serde(rename = "XAB_DEBIT")]
        XabDebit,
        /// 西安银行(信用卡)
        #[serde(rename = "XAB_CREDIT")]
        XabCredit,
        /// 汉口银行(借记卡)
        #[serde(rename = "HKB_DEBIT")]
        HkbDebit,
        /// 东莞银行(借记卡)
        #[serde(rename = "BOD_DEBIT")]
        BodDebit,
        /// 东莞银行(信用卡)
        #[serde(rename = "BOD_CREDIT")]
        BodCredit,
        /// 青岛银行(借记卡)
        #[serde(rename = "QDCCB_DEBIT")]
        QdccbDebit,
        /// 青岛银行(信用卡)
        #[serde(rename = "QDCCB_CREDIT")]
        QdccbCredit,
        /// 华润银行(借记卡)
        #[serde(rename = "CRB_DEBIT")]
        CrbDebit,
        /// 晋商银行(借记卡)
        #[serde(rename = "JSHB_DEBIT")]
        JshbDebit,
        /// 江西银行(借记卡)
        #[serde(rename = "BNC_DEBIT")]
        BncDebit,
        /// 江西银行(信用卡)
        #[serde(rename = "BNC_CREDIT")]
        BncCredit,
        /// 南粤银行(借记卡)
        #[serde(rename = "GDNYB_DEBIT")]
        GdnybDebit,
        /// 南粤银行(信用卡)
        #[serde(rename = "GDNYB_CREDIT")]
        GdnybCredit,
        /// 苏州银行(借记卡)
        #[serde(rename = "SUZB_DEBIT")]
        SuzbDebit,
        /// 温州银行(借记卡)
        #[serde(rename = "WZB_DEBIT")]
        WzbDebit,
        /// 厦门银行(借记卡)
        #[serde(rename = "XMCCB_DEBIT")]
        XmccbDebit,
        /// 厦门国际银行(借记卡)
        #[serde(rename = "XIB_DEBIT")]
        XibDebit,
        /// 台州银行(借记卡)
        #[serde(rename = "TZB_DEBIT")]
        TzbDebit,
        /// 福建海峡银行(借记卡)
        #[serde(rename = "FJHXB_DEBIT")]
        FjhxbDebit,
        /// 福建海峡银行(信用卡)
        #[serde(rename = "FJHXB_CREDIT")]
        FjhxbCredit,
        /// 民泰银行(借记卡)
        #[serde(rename = "MINTAIB_DEBIT")]
        MintaibDebit,
        /// 民泰银行(信用卡)
        #[serde(rename = "MINTAIB_CREDIT")]
        MintaibCredit,
        /// 唐山银行(借记卡)
        #[serde(rename = "BOTSB_DEBIT")]
        BotsbDebit,
        /// 廊坊银行(借记卡)
        #[serde(rename = "BOLFB_DEBIT")]
        BolfbDebit,
        /// 邯郸银行(借记卡)
        #[serde(rename = "HDCB_DEBIT")]
        HdcbDebit,
        /// 沧州银行(借记卡)
        #[serde(rename = "BCZ_DEBIT")]
        BczDebit,
        /// 沧州银行(信用卡)
        #[serde(rename = "BCZ_CREDIT")]
        BczCredit,
        /// 承德银行(借记卡)
        #[serde(rename = "BOCDB_DEBIT")]
        BocdbDebit,
        /// 营口银行(借记卡)
        #[serde(rename = "BYK_DEBIT")]
        BykDebit,
        /// 张家口市商业银行(借记卡)
        #[serde(rename = "BOZ_DEBIT")]
        BozDebit,
        /// 吉林银行(借记卡)
        #[serde(rename = "JLB_DEBIT")]
        JlbDebit,
        /// 龙江银行(借记卡)
        #[serde(rename = "LJB_DEBIT")]
        LjbDebit,
        /// 齐商银行(借记卡)
        #[serde(rename = "QSB_DEBIT")]
        QsbDebit,
        /// 锦州银行(借记卡)
        #[serde(rename = "JZCB_DEBIT")]
        JzcbDebit,
        /// 锦州银行(信用卡)
        #[serde(rename = "JZCB_CREDIT")]
        JzcbCredit,
        /// 洛阳银行(借记卡)
        #[serde(rename = "BOLB_DEBIT")]
        BolbDebit,
        /// 潍坊银行(借记卡)
        #[serde(rename = "WFB_DEBIT")]
        WfbDebit,
        /// 潍坊银行(信用卡)
        #[serde(rename = "WFB_CREDIT")]
        WfbCredit,
        /// 日照银行(借记卡)
        #[serde(rename = "RZB_DEBIT")]
        RzbDebit,
        /// 东营银行(借记卡)
        #[serde(rename = "DYB_DEBIT")]
        DybDebit,
        /// 东营银行(信用卡)
        #[serde(rename = "DYB_CREDIT")]
        DybCredit,
        /// 威海市商业银行(借记卡)
        #[serde(rename = "WHB_DEBIT")]
        WhbDebit,
        /// 威海商业银行(信用卡)
        #[serde(rename = "WHB_CREDIT")]
        WhbCredit,
        /// 临商银行(借记卡)
        #[serde(rename = "LWB_DEBIT")]
        LwbDebit,
        /// 泰安银行(借记卡)
        #[serde(rename = "TACCB_DEBIT")]
        TaccbDebit,
        /// 泰安银行(信用卡)
        #[serde(rename = "TACCB_CREDIT")]
        TaccbCredit,
        /// 丹东银行(借记卡)
        #[serde(rename = "DANDONGB_DEBIT")]
        DandongbDebit,
        /// 丹东银行(信用卡)
        #[serde(rename = "DANDONGB_CREDIT")]
        DandongbCredit,
        /// 昆仑银行(借记卡)
        #[serde(rename = "KLB_DEBIT")]
        KlbDebit,
        /// 烟台银行(借记卡)
        #[serde(rename = "YTB_DEBIT")]
        YtbDebit,
        /// 内蒙古银行(借记卡)
        #[serde(rename = "BOIMCB_DEBIT")]
        BoimcbDebit,
        /// 阜新银行(借记卡)
        #[serde(rename = "FUXINB_DEBIT")]
        FuxinbDebit,
        /// 宁夏银行(借记卡)
        #[serde(rename = "BONX_DEBIT")]
        BonxDebit,
        /// 宁夏银行(信用卡)
        #[serde(rename = "BONX_CREDIT")]
        BonxCredit,
        /// 湖商村镇银行(借记卡)
        #[serde(rename = "HUSRB_DEBIT")]
        HusrbDebit,
        /// 重庆三峡银行(借记卡)
        #[serde(rename = "CQTGB_DEBIT")]
        CqtgbDebit,
        /// 晋中银行(借记卡)
        #[serde(rename = "JZB_DEBIT")]
        JzbDebit,
        /// 晋城银行(借记卡)
        #[serde(rename = "JCB_DEBIT")]
        JcbDebit,
        /// 九江银行(借记卡)
        #[serde(rename = "JJCCB_DEBIT")]
        JjccbDebit,
        /// 嘉兴银行(借记卡)
        #[serde(rename = "BOJX_DEBIT")]
        BojxDebit,
        /// 浙江泰隆银行(借记卡)
        #[serde(rename = "ZJTLCB_DEBIT")]
        ZjtlcbDebit,
        /// 稠州银行(借记卡)
        #[serde(rename = "CZCB_DEBIT")]
        CzcbDebit,
        /// 稠州银行(信用卡)
        #[serde(rename = "CZCB_CREDIT")]
        CzcbCredit,
        /// 湖北银行(借记卡)
        #[serde(rename = "HBCB_DEBIT")]
        HbcbDebit,
        /// 湖北银行(信用卡)
        #[serde(rename = "HBCB_CREDIT")]
        HbcbCredit,
        /// 富滇银行(借记卡)
        #[serde(rename = "FDB_DEBIT")]
        FdbDebit,
        /// 桂林银行(借记卡)
        #[serde(rename = "GLB_DEBIT")]
        GlbDebit,
        /// 长安银行(借记卡)
        #[serde(rename = "CCAB_DEBIT")]
        CcabDebit,
        /// 长安银行(信用卡)
        #[serde(rename = "CCAB_CREDIT")]
        CcabCredit,
        /// 齐鲁银行(借记卡)
        #[serde(rename = "QLB_DEBIT")]
        QlbDebit,
        /// 广东华兴银行(借记卡)
        #[serde(rename = "GDHX_DEBIT")]
        GdhxDebit,
        /// 海南银行(借记卡)
        #[serde(rename = "BOHN_DEBIT")]
        BohnDebit,
        /// 上饶银行(借记卡)
        #[serde(rename = "SRB_DEBIT")]
        SrbDebit,
        /// 中原银行(借记卡)
        #[serde(rename = "ZYB_DEBIT")]
        ZybDebit,
        /// 沧州银行(借记卡)
        #[serde(rename = "BCZ_DEBIT")]
        BczDebit2,
        /// 德阳银行(借记卡)
        #[serde(rename = "DYCCB_DEBIT")]
        DyccbDebit,
        /// 恒丰银行(借记卡)
        #[serde(rename = "HFB_DEBIT")]
        HfbDebit,
        /// 华融湘江银行(借记卡)
        #[serde(rename = "HRXJB_DEBIT")]
        HrxjbDebit,
        /// 柳州银行(借记卡)
        #[serde(rename = "LUZB_DEBIT")]
        LuzbDebit,
        /// 攀枝花银行(借记卡)
        #[serde(rename = "PZHCCB_DEBIT")]
        PzhccbDebit,
        /// 鄂尔多斯银行(借记卡)
        #[serde(rename = "ORDOSB_DEBIT")]
        OrdosbDebit,
        /// 鄂尔多斯银行(信用卡)
        #[serde(rename = "ORDOSB_CREDIT")]
        OrdosbCredit,

        // ============ 农村商业银行/农村信用社 ============
        /// 北京农商行(借记卡)
        #[serde(rename = "BJRCB_DEBIT")]
        BjrcbDebit,
        /// 北京农商(信用卡)
        #[serde(rename = "BJRCB_CREDIT")]
        BjrcbCredit,
        /// 上海农商银行(借记卡)
        #[serde(rename = "SRCB_DEBIT")]
        SrcbDebit,
        /// 上海农商银行(信用卡)
        #[serde(rename = "SRCB_CREDIT")]
        SrcbCredit,
        /// 广州农商银行(借记卡)
        #[serde(rename = "GRCB_DEBIT")]
        GrcbDebit,
        /// 广州农商银行(信用卡)
        #[serde(rename = "GRCB_CREDIT")]
        GrcbCredit,
        /// 深圳农商银行(借记卡)
        #[serde(rename = "SZRCB_DEBIT")]
        SzrcbDebit,
        /// 深圳农商银行(信用卡)
        #[serde(rename = "SZRCB_CREDIT")]
        SzrcbCredit,
        /// 东莞农商银行(借记卡)
        #[serde(rename = "DRCB_DEBIT")]
        DrcbDebit,
        /// 东莞农商银行(信用卡)
        #[serde(rename = "DRCB_CREDIT")]
        DrcbCredit,
        /// 顺德农商行(借记卡)
        #[serde(rename = "SDEB_DEBIT")]
        SdebDebit,
        /// 重庆农商银行(借记卡)
        #[serde(rename = "CQRCB_DEBIT")]
        CqrcbDebit,
        /// 重庆农商银行(信用卡)
        #[serde(rename = "CQRCB_CREDIT")]
        CqrcbCredit,
        /// 成都农商银行(借记卡)
        #[serde(rename = "CDRCB_DEBIT")]
        CdrbDebit,
        /// 江南农商(借记卡)
        #[serde(rename = "JNRCB_DEBIT")]
        JnrcbDebit,
        /// 昆山农商(借记卡)
        #[serde(rename = "KRCB_DEBIT")]
        KrcbDebit,
        /// 常熟农商银行(借记卡)
        #[serde(rename = "CSRCB_DEBIT")]
        CsrcbDebit,
        /// 常熟农商银行(信用卡)
        #[serde(rename = "CSRCB_CREDIT")]
        CsrcbCredit,
        /// 无锡农商(借记卡)
        #[serde(rename = "WRCB_DEBIT")]
        WrcbDebit,
        /// 张家港农商行(借记卡)
        #[serde(rename = "ZRCB_DEBIT")]
        ZrcbDebit,
        /// 天津农商(借记卡)
        #[serde(rename = "TRCB_DEBIT")]
        TrcbDebit,
        /// 天津滨海农商行(借记卡)
        #[serde(rename = "TJBHB_DEBIT")]
        TjbhbDebit,
        /// 天津滨海农商行(信用卡)
        #[serde(rename = "TJBHB_CREDIT")]
        TjbhbCredit,
        /// 武汉农商行(借记卡)
        #[serde(rename = "WHRC_DEBIT")]
        WhrcDebit,
        /// 武汉农商(信用卡)
        #[serde(rename = "WHRC_CREDIT")]
        WhrcCredit,
        /// 江阴农商行(借记卡)
        #[serde(rename = "JRCB_DEBIT")]
        JrcbDebit,
        /// 紫金农商银行(借记卡)
        #[serde(rename = "ZJB_DEBIT")]
        ZjbDebit,
        /// 太仓农商行(借记卡)
        #[serde(rename = "TCRCB_DEBIT")]
        TcrcbDebit,
        /// 吴江农商行(借记卡)
        #[serde(rename = "WJRCB_DEBIT")]
        WjrcbDebit,
        /// 江苏农商行(借记卡)
        #[serde(rename = "JSNX_DEBIT")]
        JsnxDebit,
        /// 浙江农信(借记卡)
        #[serde(rename = "ZJRCUB_DEBIT")]
        ZjrcubDebit,
        /// 浙江农信(信用卡)
        #[serde(rename = "ZJRCUB_CREDIT")]
        ZjrcubCredit,
        /// 广东农信银行(借记卡)
        #[serde(rename = "GDRCU_DEBIT")]
        GdrcuDebit,
        /// 河南农信(借记卡)
        #[serde(rename = "HNNX_DEBIT")]
        HnnxDebit,
        /// 湖北农信(借记卡)
        #[serde(rename = "HBNX_DEBIT")]
        HbnxDebit,
        /// 湖北农信(信用卡)
        #[serde(rename = "HBNX_CREDIT")]
        HbnxCredit,
        /// 湖南农信(借记卡)
        #[serde(rename = "HUNNX_DEBIT")]
        HunnxDebit,
        /// 广西农信(借记卡)
        #[serde(rename = "GXNX_DEBIT")]
        GxnxDebit,
        /// 四川农信(借记卡)
        #[serde(rename = "SCNX_DEBIT")]
        ScnxDebit,
        /// 贵州农信(借记卡)
        #[serde(rename = "GZNX_DEBIT")]
        GznxDebit,
        /// 云南农信(借记卡)
        #[serde(rename = "YNRCCB_DEBIT")]
        YnrcbDebit,
        /// 陕西信合(借记卡)
        #[serde(rename = "SXXH_DEBIT")]
        SxxhDebit,
        /// 河北农信(借记卡)
        #[serde(rename = "HEBNX_DEBIT")]
        HebnxDebit,
        /// 山西农信(借记卡)
        #[serde(rename = "SXNX_DEBIT")]
        SxnxDebit,
        /// 内蒙古农信(借记卡)
        #[serde(rename = "NMGNX_DEBIT")]
        NmgnxDebit,
        /// 辽宁农信(借记卡)
        #[serde(rename = "LNNX_DEBIT")]
        LnnxDebit,
        /// 吉林农信(借记卡)
        #[serde(rename = "JLNX_DEBIT")]
        JlnxDebit,
        /// 黑龙江农信(借记卡)
        #[serde(rename = "HLJNX_DEBIT")]
        HljnxDebit,
        /// 江苏农信(借记卡)
        #[serde(rename = "JSNX_DEBIT")]
        Jsnx2Debit,
        /// 安徽农信(借记卡)
        #[serde(rename = "AHNX_DEBIT")]
        AhnxDebit,
        /// 福建农信银行(借记卡)
        #[serde(rename = "FJNX_DEBIT")]
        FjnxDebit,
        /// 江西农信(借记卡)
        #[serde(rename = "JXNXB_DEBIT")]
        JxnxbDebit,
        /// 山东农信(借记卡)
        #[serde(rename = "SDRCU_DEBIT")]
        SdrcuDebit,
        /// 海南农信(借记卡)
        #[serde(rename = "HAINNX_DEBIT")]
        HainnxDebit,
        /// 海南农信(信用卡)
        #[serde(rename = "HAINNX_CREDIT")]
        HainnxCredit,
        /// 甘肃农信(借记卡)
        #[serde(rename = "GSNX_DEBIT")]
        GsnxDebit,
        /// 青海农信(借记卡)
        #[serde(rename = "QHNX_DEBIT")]
        QhnxDebit,
        /// 新疆农信银行(借记卡)
        #[serde(rename = "XJRCCB_DEBIT")]
        XjrccbDebit,
        /// 黄河农商银行(借记卡)
        #[serde(rename = "YRRCB_DEBIT")]
        YrrcbDebit,
        /// 黄河农商银行(信用卡)
        #[serde(rename = "YRRCB_CREDIT")]
        YrrcbCredit,

        // ============ 外资/合资银行 ============
        /// 东亚银行(借记卡)
        #[serde(rename = "HKBEA_DEBIT")]
        HkbeaDebit,
        /// 恒生银行(借记卡)
        #[serde(rename = "HSBC_DEBIT")]
        HsbcDebit,
        /// 新韩银行(借记卡)
        #[serde(rename = "SHINHAN_DEBIT")]
        ShinhanDebit,
        /// 韩亚银行(借记卡)
        #[serde(rename = "HANAB_DEBIT")]
        HanabDebit,
        /// 富邦华一银行(借记卡)
        #[serde(rename = "FBB_DEBIT")]
        FbbDebit,

        // ============ 其他 ============
        /// 微众银行(借记卡)
        #[serde(rename = "WEB_DEBIT")]
        WebDebit,
        /// 联合村镇银行(借记卡)
        #[serde(rename = "URB_DEBIT")]
        UrbDebit,
        /// 南阳村镇银行(借记卡)
        #[serde(rename = "NYCCB_DEBIT")]
        NyccbDebit,
        /// 临朐聚丰村镇银行(借记卡)
        #[serde(rename = "JUFENGB_DEBIT")]
        JufengbDebit,
        /// 兰溪越商银行(借记卡)
        #[serde(rename = "ZJLXRB_DEBIT")]
        ZjlxrbDebit,
        /// 鄞州银行(借记卡)
        #[serde(rename = "BEEB_DEBIT")]
        BeebDebit,
        /// 鄞州银行(信用卡)
        #[serde(rename = "BEEB_CREDIT")]
        BeebCredit,
        /// 绍兴银行(借记卡)
        #[serde(rename = "BOSXB_DEBIT")]
        BosxbDebit,
        /// 石嘴山银行(借记卡)
        #[serde(rename = "BOSZS_DEBIT")]
        BoszsDebit,
        /// 库尔勒市商业银行(借记卡)
        #[serde(rename = "KUERLECB_DEBIT")]
        KuerlecbDebit,

        // ============ 国际卡组织 ============
        /// JCB(信用卡)
        #[serde(rename = "JCB_CREDIT")]
        JcbCredit,
        /// MASTERCARD(信用卡)
        #[serde(rename = "MASTERCARD_CREDIT")]
        MastercardCredit,
        /// VISA(信用卡)
        #[serde(rename = "VISA_CREDIT")]
        VisaCredit,
        /// AE(信用卡)
        #[serde(rename = "AE_CREDIT")]
        AeCredit,

        // ============ 第三方支付 ============
        /// 零钱/零钱通
        #[serde(rename = "OTHERS")]
        Others,
        /// 云闪付（借记卡）
        #[serde(rename = "UPQUICKPASS_DEBIT")]
        UpquickpassDebit,
        /// 云闪付（信用卡）
        #[serde(rename = "UPQUICKPASS_CREDIT")]
        UpquickpassCredit,
    }
}
