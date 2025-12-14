use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WeatherInfo {
    pub lat: f64,
    pub lon: f64,
    pub timezone: String,
    pub timezone_offset: i32,
    pub current: CurrentWeather,
    pub minutely: Vec<MinutelyForecast>,
    pub hourly: Vec<HourlyForecast>,
    pub daily: Vec<DailyForecast>,
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Deserialize)]
pub struct CurrentWeather {
    pub dt: i64,
    pub sunrise: i64,
    pub sunset: i64,
    pub temp: f64,
    pub feels_like: f64,
    pub pressure: i32,
    pub humidity: i32,
    pub dew_point: f64,
    pub uvi: f64,
    pub clouds: i32,
    pub visibility: i32,
    pub wind_speed: f64,
    pub wind_deg: i32,
    pub wind_gust: f64,
    pub weather: Vec<WeatherCondition>,
}

#[derive(Debug, Deserialize)]
pub struct MinutelyForecast {
    pub dt: i64,
    pub precipitation: f64,
}

#[derive(Debug, Deserialize)]
pub struct HourlyForecast {
    pub dt: i64,
    pub temp: f64,
    pub feels_like: f64,
    pub pressure: i32,
    pub humidity: i32,
    pub dew_point: f64,
    pub uvi: f64,
    pub clouds: i32,
    pub visibility: i32,
    pub wind_speed: f64,
    pub wind_deg: i32,
    pub wind_gust: f64,
    pub weather: Vec<WeatherCondition>,
    pub pop: f64,
}

#[derive(Debug, Deserialize)]
pub struct DailyForecast {
    pub dt: i64,
    pub sunrise: i64,
    pub sunset: i64,
    pub moonrise: i64,
    pub moonset: i64,
    pub moon_phase: f64,
    pub summary: String,
    pub temp: Temperature,
    pub feels_like: FeelsLike,
    pub pressure: i32,
    pub humidity: i32,
    pub dew_point: f64,
    pub wind_speed: f64,
    pub wind_deg: i32,
    pub wind_gust: f64,
    pub weather: Vec<WeatherCondition>,
    pub clouds: i32,
    pub pop: f64,
    pub rain: f64,
    pub uvi: f64,
}

#[derive(Debug, Deserialize)]
pub struct Temperature {
    pub day: f64,
    pub min: f64,
    pub max: f64,
    pub night: f64,
    pub eve: f64,
    pub morn: f64,
}

#[derive(Debug, Deserialize)]
pub struct FeelsLike {
    pub day: f64,
    pub night: f64,
    pub eve: f64,
    pub morn: f64,
}

#[derive(Debug, Deserialize)]
pub struct WeatherCondition {
    pub id: i32,
    pub main: String,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Deserialize)]
pub struct Alert {
    pub sender_name: String,
    pub event: String,
    pub start: i64,
    pub end: i64,
    pub description: String,
    pub tags: Vec<String>,
}
