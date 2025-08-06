use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct WeatherInfo {
    pub latitude: f32,
    pub longitude: f32,
    pub generationtime_ms: f64,
    pub utc_offset_seconds: u64,
    pub timezone: Box<str>,
    pub timezone_abbreviation: Box<str>,
    pub elevation: f32,
    pub current_units: Option<serde_json::Map<String, Value>>,
    pub current: Option<serde_json::Map<String, Value>>,
    pub hourly_units:  Option<serde_json::Map<String, Value>>,
    pub hourly: Option<Vec<serde_json::Map<String, Value>>>,
    pub daily_units:  Option<serde_json::Map<String, Value>>,
    pub daily: Option<Vec<serde_json::Map<String, Value>>>,
}