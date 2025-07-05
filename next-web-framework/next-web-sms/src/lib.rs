pub mod core;

#[cfg(feature = "aliyun")]
pub mod aliyun;
#[cfg(feature = "tencent")]
pub mod tencent;

/// 匹配手机号码的模块
pub(crate) mod macthes {
    use once_cell::sync::Lazy;
    use regex::Regex;
    use std::sync::Arc;

    /// 中国大陆手机号码正则
    pub static MAINLAND_MOBILE: Lazy<Arc<Regex>> =
        Lazy::new(|| Arc::new(Regex::new(r"^(?:\+?86)?1[3-9]\d{9}$").unwrap()));

    /// 香港手机号码正则
    pub static HONG_KONG_MOBILE: Lazy<Arc<Regex>> =
        Lazy::new(|| Arc::new(Regex::new(r"^(?:\+?852\-?)?[569]\d{7}$").unwrap()));

    /// 台湾手机号码正则
    pub static TAIWAN_REGEX: Lazy<Arc<Regex>> =
        Lazy::new(|| Arc::new(Regex::new(r"^(?:\+?886\-?|0)?9\d{8}$").unwrap()));

    /// 澳门手机号码正则
    pub static MACAU_REGEX: Lazy<Arc<Regex>> =
        Lazy::new(|| Arc::new(Regex::new(r"^(?:\+?886\-?|0)?9\d{8}$").unwrap()));
}