use serde::Deserialize;

use crate::Named;

#[derive(Debug, Clone, Deserialize)]
pub struct FacePayQueryResponse {
    /// 支付宝uid
    ///
    /// 新商户建议使用open_id替代该字段。对于新商户，user_id字段未来计划逐步回收，存量商户可继续使用。
    /// 如使用open_id，请确认 应用-开发配置-openid配置管理 已启用。无该配置项
    pub uid: Option<String>,

    /// 支付宝用户open_id
    pub open_id: String,

    /// 用户名信息返回的列表
    pub uid_tel_pair_list: Option<Vec<ZhubUidTelPair>>,

    /// 年龄是否在指定范围内，未指定范围则返回空，true/false
    pub age_check_result: Option<String>,

    /// 身份证号码
    pub cert_no: Option<String>,

    /// 证件姓名
    pub cert_name: Option<String>,

    /// 由ISV定义的对自然人唯一编码，举例可以是身份证号码和姓名的MD5值，或者是其他编码方式，要求脱敏、随机且在ISV可以唯一说明一个自然人
    pub face_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZhubUidTelPair {
    /// 支付宝uid
    pub user_id: Option<String>,

    /// 支付宝用户open_id
    pub open_id: String,
}

impl Named for FacePayQueryResponse {
    fn name() -> &'static str {
        "zoloz_authentication_customer_ftoken_query_response"
    }
}
