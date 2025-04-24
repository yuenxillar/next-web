use serde::Deserialize;
use tracing::{event, Level};

use super::error::ErrorCode;
use crate::Result;


/// 微信小程序返回的数据结构
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Response<T> {
    Success {
        #[serde(flatten)]
        data: T,
    },
    Error {
        #[serde(rename = "errcode")]
        code: ErrorCode,
        #[serde(rename = "errmsg")]
        message: String,
    },
}

impl<T> Response<T> {
    /// 获取微信小程序返回的数据
    pub fn extract(self) -> Result<T> {
        match self {
            Self::Success { data } => Ok(data),
            Self::Error { code, message } => {
                event!(
                    Level::ERROR,
                    "微信小程序返回错误: code={}, message={}",
                    code,
                    message
                );

                Err((code, message).into())
            }
        }
    }
}