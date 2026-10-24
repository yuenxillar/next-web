use serde::Serialize;

use crate::Method;

/// 刷脸支付初始化请求参数
///
/// 包含设备信息、生物识别信息、商户信息等刷脸支付所需的完整参数
#[derive(Debug, Clone, Serialize, Default)]
pub struct FacePayInitializeRequest {
    /// 设备指纹，用于唯一标识一台设备
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apdid_token: Option<String>,

    /// 生物识别元信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio_meta_info: Option<String>,

    /// 人脸识别应用名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,

    /// 人脸识别应用版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,

    /// 设备类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,

    /// 设备型号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,

    /// 操作系统版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,

    /// ZIM 版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zim_ver: Option<String>,

    /// 基础包版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ver: Option<String>,

    /// 业务 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_log_id: Option<String>,

    /// 机具信息
    ///
    /// 由调用人脸识别 SDK 获取，包含摄像头、机具码等硬件信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_info: Option<FaceMachineInfo>,

    /// 商户信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_info: Option<FaceMerchantInfo>,

    /// 扩展信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext_info: Option<FaceExtParams>,
}

/// 机具信息
///
/// 刷脸支付设备的硬件相关信息，包括摄像头、机具编码等
#[derive(Debug, Clone, Serialize)]
pub struct FaceMachineInfo {
    /// 摄像头驱动版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_drive_ver: Option<String>,

    /// 摄像头型号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_model: Option<String>,

    /// 摄像头名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_name: Option<String>,

    /// 摄像头版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_ver: Option<String>,

    /// 机具编码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_code: Option<String>,

    /// 机具型号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_model: Option<String>,

    /// 机具版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_ver: Option<String>,

    /// 扩展信息
    pub ext: Option<String>,
}

/// 商户信息
///
/// 刷脸支付设备所在商户的相关标识信息
#[derive(Debug, Clone, Serialize)]
pub struct FaceMerchantInfo {
    /// 区域编码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_code: Option<String>,

    /// 品牌编码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand_code: Option<String>,

    /// 机具 MAC 地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_mac: Option<String>,

    /// 机具分组编码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,

    /// 机具编码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_num: Option<String>,

    /// 经纬度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<String>,

    /// 商户 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_id: Option<String>,

    /// ISV ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_id: Option<String>,

    /// 门店编码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_code: Option<String>,

    /// WiFi MAC 地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifimac: Option<String>,

    /// WiFi 名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifiname: Option<String>,
}

/// 扩展信息
#[derive(Debug, Clone, Serialize)]
pub struct FaceExtParams {
    /// 业务类型
    ///
    /// - 7: 基于 1:N 人脸搜索的刷脸支付场景
    /// - 8: 基于姓名和身份证号的刷脸支付场景
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biz_type: Option<String>,
}

impl Method for FacePayInitializeRequest {
    fn method() -> &'static str {
        "zoloz.authentication.smilepay.initialize"
    }
}
