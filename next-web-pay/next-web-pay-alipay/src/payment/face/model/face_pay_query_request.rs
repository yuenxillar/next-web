use serde::Serialize;

use crate::Method;

#[derive(Debug, Clone, Serialize)]
pub struct FacePayQueryRequest {
    /// 人脸token
    /// 示例值: fp0593e8d5c136277f13fd5bc36c13a7db7
    ftoken: String,

    /// 1：1人脸验证能力
    /// 2、1：n人脸搜索能力（支付宝uid入库）
    /// 3、1：n人脸搜索能力（支付宝手机号入库）
    /// 4、手机号和人脸识别综合能力
    biz_type: String,

    /// 刷脸初始化流程中产生的zimId值
    /// 3b35b4677de2c69bb5bab69a4a5168d62
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zim_id: Option<String>,

    /// 人脸产品拓展参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext_info: Option<FaceExtInfo>,
}

impl FacePayQueryRequest {
    pub fn new(ftoken: impl Into<String>, biz_type: impl Into<String>) -> Self {
        Self {
            ftoken: ftoken.into(),
            biz_type: biz_type.into(),
            zim_id: None,
            ext_info: None,
        }
    }

    pub fn with_zim_id(mut self, zim_id: impl Into<String>) -> Self {
        self.zim_id = Some(zim_id.into());

        self
    }

    pub fn with_ext_info(mut self, ext_info: FaceExtInfo) -> Self {
        self.ext_info = Some(ext_info);

        self
    }
}

/// 人脸产品拓展参数.
#[derive(Debug, Clone, Serialize)]
pub struct FaceExtInfo {
    /// 年龄区间判断的下限，闭区间
    #[serde(skip_serializing_if = "Option::is_none")]
    min_age: Option<String>,

    /// 年龄区间判断的上限，闭区间
    #[serde(skip_serializing_if = "Option::is_none")]
    max_age: Option<String>,
}

impl Method for FacePayQueryRequest {
    fn method() -> &'static str {
        "zoloz.authentication.customer.ftoken.query"
    }
}
