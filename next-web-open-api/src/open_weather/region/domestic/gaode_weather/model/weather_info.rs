use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct WeatherInfo {
    /// Total number of returned results
    ///
    /// 返回结果总数目
    pub count: Box<str>,

    /// Live weather data information
    ///
    /// 实况天气数据信息
    pub lives: Vec<Live>,
    
    /// Forecast weather information data
    ///
    /// 预报天气信息数据
    pub forecast: Vec<Forecast>,
}

#[derive(Debug, Deserialize)]
pub struct Live {
    ///
    ///
    /// 省份名
    pub province: Box<str>,
    ///
    ///
    /// 城市名
    pub city: Box<str>,
    ///
    ///
    /// 区域编码
    pub adcode: Box<str>,
    ///
    ///
    /// 天气现象 （汉字描述）
    pub weather: Box<str>,
    ///
    ///
    /// 实时气温，单位：摄氏度
    pub temperature: Box<str>,
    ///
    ///
    /// 风向描述
    pub winddirection: Box<str>,
    ///
    ///
    /// 风力级别，单位：级
    pub windpower: Box<str>,
    ///
    ///
    /// 空气湿度
    pub humidity: Box<str>,
    ///
    ///
    /// 数据发布的时间
    pub reporttime: Box<str>,
    ///
    ///
    /// 温度
    pub temperature_float: Option<Box<str>>,
    ///
    ///
    /// 湿度
    pub humidity_float: Option<Box<str>>,
}

#[derive(Debug, Deserialize)]
pub struct Forecast {
    ///
    ///
    /// 城市名称
    pub city: Box<str>,
    ///
    ///
    /// 城市编码
    pub adcode: Box<str>,
    ///
    ///
    /// 省份名称
    pub province: Box<str>,
    ///
    ///
    /// 预报发布时间
    pub reporttime: Box<str>,
    ///
    ///
    /// 预报数据
    pub casts: Vec<Cast>,
}

/// 预报天气信息数据
#[derive(Debug, Deserialize)]
pub struct Cast {
    ///
    ///
    /// 日期
    pub date: Box<str>,
    ///
    ///
    /// 星期
    pub week: Box<str>,
    ///
    ///
    /// 白天天气现象
    pub dayweather: Box<str>,
    ///
    ///
    /// 晚上天气现象
    pub nightweather: Box<str>,
    ///
    ///
    /// 白天温度
    pub daytemp: Box<str>,
    ///
    ///
    /// 晚上温度
    pub nighttemp: Box<str>,
    ///
    ///
    /// 白天风向
    pub daywind: Box<str>,
    ///
    ///
    /// 晚上风向
    pub nightwind: Box<str>,
    ///
    ///
    /// 白天风力
    pub daypower: Box<str>,
    ///
    ///
    /// 晚上风力
    pub nightpower: Box<str>,
}
