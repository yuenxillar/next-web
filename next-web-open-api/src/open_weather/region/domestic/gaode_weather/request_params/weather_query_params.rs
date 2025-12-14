use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct WeatherQueryParams {
    /// City code: Enter the adcode of the city
    ///
    /// 城市编码 输入城市的 adcode
    /// https://lbs.amap.com/api/webservice/download
    pub city: u32,
    /// Request service permission identifier
    ///
    /// 请求服务权限标识
    pub(crate) key: Option<Box<str>>,
    /// Meteorological type
    /// Optional value: base/all
    /// Base: Return real-time weather all: Return forecast weather
    ///
    /// 气象类型
    /// 可选值：base/all
    /// base:返回实况天气 all:返回预报天气
    pub extensions: Option<Box<str>>,
    /// Return format JSON XML
    ///
    /// 返回格式 JSON XML
    pub output: Option<Box<str>>,
}

impl WeatherQueryParams {
    /// Defualt JSON format
    ///
    /// 默认JSON格式
    pub fn new(city: u32) -> Self {
        Self {
            city,
            key: None,
            extensions: None,
            output: Some("JSON".into()),
        }
    }
}
