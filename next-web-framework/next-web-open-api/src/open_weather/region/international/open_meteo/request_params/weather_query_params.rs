use serde::Serialize;

/// https://open-meteo.com/en/docs
#[derive(Clone, Serialize)]
pub struct WeatherQueryParams {
    pub latitude: f32,
    pub longitude: f32,
    #[serde(skip)]
    pub hourly: Vec<(WeatherType, Vec<Box<str>>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeatherType {
    CurrentWeather,
    DailyWeatherVariables,
    HourlyWeatherVariables,
    All,
}

impl WeatherQueryParams {
    pub fn new(latitude: f32, longitude: f32, hourly: Vec<(WeatherType, Vec<Box<str>>)>) -> Self {
        Self {
            latitude,
            longitude,
            hourly,
        }
    }
}