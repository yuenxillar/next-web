use serde::Deserialize;

use crate::Named;

#[derive(Debug, Clone, Deserialize)]
pub struct FacePayInitializeResponse {
    /// 返回详细码
    /// 示例值: Z3161
    pub ret_code_sub: String,

    /// 返回详细信息
    /// 示例值: 操作成功
    pub ret_message_sub: String,

    /// ZIM上下文ID
    pub zim_id: String,

    /// 客户端协议
    pub zim_init_client_data: String,
}


impl Named for FacePayInitializeResponse  {
    fn name() -> &'static str {
        "zoloz_authentication_smilepay_initialize_response"
    }
}