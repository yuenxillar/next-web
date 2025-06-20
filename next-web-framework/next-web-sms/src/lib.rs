pub mod core;
pub mod signature;

#[cfg(feature = "aliyun")]
pub mod aliyun;
#[cfg(feature = "tencent")]
pub mod tencent;