use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WeatherResponse<T> {
    /// Return status
    /// Value 0 or 1: Success; 0: Failed
    ///
    /// 返回状态
    /// 值为0或1 1：成功；0：失败
    pub status: Box<str>,

    /// The returned status information is OK
    ///
    /// 返回的状态信息 OK
    pub info: Box<str>,

    /// Return status explanation: 10000 represents correctness
    ///
    /// 返回状态说明 10000代表正确
    pub infocode: Box<str>,
    /// Data T
    /// 
    /// 数据 T
    #[serde(flatten)]
    pub data: Option<T>
}

impl<T> WeatherResponse<T> {
    pub fn is_ok(&self) -> bool {
        self.status.as_ref() == "1" && self.infocode.as_ref() == "10000"
    }
}
