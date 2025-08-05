use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct WeatherQueryParams {
    pub lat: f32,
    pub lon: f32,
    pub appid: Option<Box<str>>,
    pub exclude: Option<Box<str>>,
    pub units: Option<Box<str>>,
    pub lang: Option<Box<str>>,
}

impl WeatherQueryParams {
    pub fn new(lat: f32, lon: f32) -> Self {
        Self {
            lat,
            lon,
            appid: None,
            exclude: None,
            units: None,
            lang: None,
        }
    }
}
