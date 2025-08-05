#[cfg(feature = "open-weather")]
pub mod open_weather;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
