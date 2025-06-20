use serde::{Serialize, Deserialize};

/// 短信模板请求结构体
#[derive(Serialize, Debug, Clone)]
pub struct CreateSmsTemplateRequest {
    /// 模板名称，最长30个字符
    #[serde(rename = "TemplateName")]
    pub template_name: String,

    /// 模板内容，最长500个字符
    #[serde(rename = "TemplateContent")]
    pub template_content: String,

    /// 业务场景描述或线上链接，最长500个字符（可选）
    #[serde(rename = "Remark", skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,

    /// 短信类型：0=验证码，1=通知，2=推广，3=国际/港澳台
    #[serde(rename = "TemplateType")]
    pub template_type: i32, // 枚举值：0,1,2,3

    /// 关联签名名称，必须为已审核通过的签名
    #[serde(rename = "RelatedSignName")]
    pub related_sign_name: String,

    /// 模板变量规则，例如 {"code":"characterWithNumber"}
    #[serde(rename = "TemplateRule")]
    pub template_rule: String,

    /// 更多资料，上传文件时可用（可选）
    #[serde(rename = "MoreData", skip_serializing_if = "Option::is_none")]
    pub more_data: Option<Vec<String>>,

    /// 应用场景内容，如网站链接（可选）
    #[serde(rename = "ApplySceneContent", skip_serializing_if = "Option::is_none")]
    pub apply_scene_content: Option<String>,

    /// 国际/港澳台模板类型，当 TemplateType 为 3 时必填
    #[serde(rename = "IntlType", skip_serializing_if = "Option::is_none")]
    pub intl_type: Option<i32>,
}


#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct CreateSmsTemplateRespnose {

    /// 短信模板名称
    pub template_name: String,

    /// 短信模板 Code
    pub template_code: String,

    /// 工单号。
    /// 审核人员查询审核时会用到此参数。您需要审核加急时需要提供此工单号。
    pub order_id: String,
}