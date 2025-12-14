use serde::Serialize;

/// https://open-meteo.com/en/docs
#[derive(Clone, Serialize)]
pub struct WeatherQueryParams {
    pub latitude: f32,
    pub longitude: f32,
    #[serde(skip)]
    pub options: Vec<WeatherType>,
}

impl WeatherQueryParams {
    pub fn new(latitude: f32, longitude: f32, options: Vec<WeatherType>) -> Self {
        Self {
            latitude,
            longitude,
            options,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeatherType {
    CurrentWeather(Vec<CurrentVariables>),
    DailyWeatherVariables(Vec<DailyVariables>),
    HourlyWeatherVariables(Vec<HourlyVariables>),
}

impl ToString for WeatherType {
    fn to_string(&self) -> String {
        match self {
            WeatherType::CurrentWeather(items) => format!(
                "{}={}",
                self.as_ref(),
                items
                    .iter()
                    .map(|item| item.as_ref())
                    .collect::<Vec<&str>>()
                    .join(",")
            ),
            WeatherType::DailyWeatherVariables(items) => format!(
                "{}={}",
                self.as_ref(),
                items
                    .iter()
                    .map(|item| item.as_ref())
                    .collect::<Vec<&str>>()
                    .join(",")
            ),
            WeatherType::HourlyWeatherVariables(items) => format!(
                "{}={}",
                self.as_ref(),
                items
                    .iter()
                    .map(|item| item.as_ref())
                    .collect::<Vec<&str>>()
                    .join(",")
            ),
        }
    }
}
impl AsRef<str> for WeatherType {
    fn as_ref(&self) -> &str {
        match self {
            WeatherType::CurrentWeather(_) => "current",
            WeatherType::DailyWeatherVariables(_) => "daily",
            WeatherType::HourlyWeatherVariables(_) => "hourly",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]

pub enum CurrentVariables {
    /// Air temperature at 2 meters above ground (°C)
    Temperature2m,
    /// Relative humidity at 2 meters above ground (%)
    RelativeHumidity2m,
    /// Perceived temperature (°C), combining temperature, humidity, wind, and solar radiation
    ApparentTemperature,
    /// Indicator if it is currently day (1.0) or night (0.0)
    IsDay,
    /// Total precipitation (rain, showers, snow) since the last weather update (mm)
    Precipitation,
    /// Rain volume since the last weather update (mm)
    Rain,
    /// Shower volume since the last weather update (mm)
    Showers,
    /// Snowfall volume since the last weather update (cm)
    Snowfall,
    /// Weather condition code (e.g., clear, cloudy, rain, snow)
    WeatherCode,
    /// Total cloud cover (%)
    CloudCover,
    /// Atmospheric pressure at mean sea level (hPa)
    PressureMsl,
    /// Atmospheric pressure at the surface (hPa)
    SurfacePressure,
    /// Wind speed at 10 meters above ground (km/h)
    WindSpeed10m,
    /// Wind direction at 10 meters above ground (°, 0-360)
    WindDirection10m,
    /// Wind gust speed at 10 meters above ground (km/h)
    WindGusts10m,
}

impl AsRef<str> for CurrentVariables {
    fn as_ref(&self) -> &str {
        match self {
            CurrentVariables::Temperature2m => "temperature_2m",
            CurrentVariables::RelativeHumidity2m => "relative_humidity_2m",
            CurrentVariables::ApparentTemperature => "apparent_temperature",
            CurrentVariables::IsDay => "is_day",
            CurrentVariables::Precipitation => "precipitation",
            CurrentVariables::Rain => "rain",
            CurrentVariables::Showers => "showers",
            CurrentVariables::Snowfall => "snowfall",
            CurrentVariables::WeatherCode => "weather_code",
            CurrentVariables::CloudCover => "cloud_cover",
            CurrentVariables::PressureMsl => "pressure_msl",
            CurrentVariables::SurfacePressure => "surface_pressure",
            CurrentVariables::WindSpeed10m => "wind_speed_10m",
            CurrentVariables::WindDirection10m => "wind_direction_10m",
            CurrentVariables::WindGusts10m => "wind_gusts_10m",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HourlyVariables {
    /// Temperature at 2 meters above ground
    Temperature2m,
    /// Relative humidity at 2 meters above ground
    RelativeHumidity2m,
    /// Dew point temperature at 2 meters above ground
    Dewpoint2m,
    /// Apparent temperature (feels like temperature)
    ApparentTemperature,
    /// Probability of precipitation
    PrecipitationProbability,
    /// Precipitation (rain, showers, snow)
    PrecipitationRainShowersSnow,
    // Rain
    Rain,
    /// Showers
    Showers,
    /// Snowfall
    Snowfall,
    // Snow depth
    SnowDepth,
    /// Weather code (e.g., clear, cloudy, rain, etc.)
    WeatherCode,
    /// Pressure at sea level
    SealevelPressure,
    /// Surface pressure
    SurfacePressure,
    /// Cloud cover total
    CloudCover,
    /// Cloud cover at low levels
    CloudCoverLow,
    /// Cloud cover at mid levels
    CloudCoverMid,
    /// Cloud cover at high levels
    CloudCoverHigh,
    /// Visibility
    Visibility,
    /// Evapotranspiration
    Evapotranspiration,
    /// Reference evapotranspiration (ET0)
    ReferenceEvapotranspirationET0,
    /// Vapor pressure deficit
    VapourPressureDeficit,
    /// Wind speed at 10 meters
    WindSpeed10m,
    /// Wind speed at 80 meters
    WindSpeed80m,
    /// Wind speed at 120 meters
    WindSpeed120m,
    /// Wind speed at 180 meters
    WindSpeed180m,
    /// Wind direction at 10 meters
    WindDirection10m,
    /// Wind direction at 80 meters
    WindDirection80m,
    /// Wind direction at 120 meters
    WindDirection120m,
    /// Wind direction at 180 meters
    WindDirection180m,
    // Wind gusts
    WindGusts10m,
    /// Temperature at 80 meters above ground
    Temperature80m,
    /// Temperature at 120 meters above ground
    Temperature120m,
    /// Temperature at 180 meters above ground
    Temperature180m,
    /// Soil temperature at 0 cm depth
    SoilTemperature0Cm,
    /// Soil temperature at 6 cm depth
    SoilTemperature6Cm,
    /// Soil temperature at 18 cm depth
    SoilTemperature18Cm,
    /// Soil temperature at 54 cm depth
    SoilTemperature54Cm,
    /// Soil moisture in the 0-1 cm depth layer
    SoilMoisture0To1Cm,
    /// Soil moisture in the 1-3 cm depth layer
    SoilMoisture1To3Cm,
    /// Soil moisture in the 3-9 cm depth layer
    SoilMoisture3To9Cm,
    /// Soil moisture in the 9-27 cm depth layer
    SoilMoisture9To27Cm,
    /// Soil moisture in the 27-81 cm depth layer
    SoilMoisture27To81Cm,
}

impl AsRef<str> for HourlyVariables {
    fn as_ref(&self) -> &str {
        match self {
            HourlyVariables::Temperature2m => "temperature_2m",
            HourlyVariables::RelativeHumidity2m => "relative_humidity_2m",
            HourlyVariables::Dewpoint2m => "dewpoint_2m",
            HourlyVariables::ApparentTemperature => "apparent_temperature",
            HourlyVariables::PrecipitationProbability => "precipitation_probability",
            HourlyVariables::PrecipitationRainShowersSnow => "precipitation",
            HourlyVariables::Rain => "rain",
            HourlyVariables::Showers => "showers",
            HourlyVariables::Snowfall => "snowfall",
            HourlyVariables::SnowDepth => "snow_depth",
            HourlyVariables::WeatherCode => "weather_code",
            HourlyVariables::SealevelPressure => "pressure_msl",
            HourlyVariables::SurfacePressure => "surface_pressure",
            HourlyVariables::CloudCover => "cloud_cover",
            HourlyVariables::CloudCoverLow => "cloud_cover_low",
            HourlyVariables::CloudCoverMid => "cloud_cover_mid",
            HourlyVariables::CloudCoverHigh => "cloud_cover_high",
            HourlyVariables::Visibility => "visibility",
            HourlyVariables::Evapotranspiration => "evapotranspiration",
            HourlyVariables::ReferenceEvapotranspirationET0 => "et0_fao_evapotranspiration",
            HourlyVariables::VapourPressureDeficit => "vapour_pressure_deficit",
            HourlyVariables::WindSpeed10m => "wind_speed_10m",
            HourlyVariables::WindSpeed80m => "wind_speed_80m",
            HourlyVariables::WindSpeed120m => "wind_speed_120m",
            HourlyVariables::WindSpeed180m => "wind_speed_180m",
            HourlyVariables::WindDirection10m => "wind_direction_10m",
            HourlyVariables::WindDirection80m => "wind_direction_80m",
            HourlyVariables::WindDirection120m => "wind_direction_120m",
            HourlyVariables::WindDirection180m => "wind_direction_180m",
            HourlyVariables::WindGusts10m => "wind_gusts_10m",
            HourlyVariables::Temperature80m => "temperature_80m",
            HourlyVariables::Temperature120m => "temperature_120m",
            HourlyVariables::Temperature180m => "temperature_180m",
            HourlyVariables::SoilTemperature0Cm => "soil_temperature_0cm",
            HourlyVariables::SoilTemperature6Cm => "soil_temperature_6cm",
            HourlyVariables::SoilTemperature18Cm => "soil_temperature_18cm",
            HourlyVariables::SoilTemperature54Cm => "soil_temperature_54cm",
            HourlyVariables::SoilMoisture0To1Cm => "soil_moisture_0_to_1cm",
            HourlyVariables::SoilMoisture1To3Cm => "soil_moisture_1_to_3cm",
            HourlyVariables::SoilMoisture3To9Cm => "soil_moisture_3_to_9cm",
            HourlyVariables::SoilMoisture9To27Cm => "soil_moisture_9_to_27cm",
            HourlyVariables::SoilMoisture27To81Cm => "soil_moisture_27_to_81cm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]

pub enum DailyVariables {
    /// Weather condition code (e.g., clear, cloudy, rain, snow)
    WeatherCode,
    /// Maximum temperature at 2 meters above ground during the hour
    Temperature2mMax,
    /// Minimum temperature at 2 meters above ground during the hour
    Temperature2mMin,
    /// Maximum apparent temperature at 2 meters above ground during the hour
    ApparentTemperatureMax,
    /// Minimum apparent temperature at 2 meters above ground during the hour
    ApparentTemperatureMin,
    /// Sunrise time for the day (UTC)
    Sunrise,
    /// Sunset time for the day (UTC)
    Sunset,
    /// Duration of daylight in seconds for the day
    DaylightDuration,
    /// Duration of sunshine in seconds for the hour
    SunshineDuration,
    /// UV index during the hour
    UvIndex,
    /// UV index during the hour under clear sky conditions
    UvIndexClearSky,
    /// Sum of rain volume during the hour (mm)
    RainSum,
    /// Sum of shower volume during the hour (mm)
    ShowersSum,
    /// Sum of snowfall volume during the hour (cm)
    SnowfallSum,
    /// Sum of total precipitation volume during the hour (mm)
    PrecipitationSum,
    /// Number of hours with significant precipitation (fractional, 0-1)
    PrecipitationHours,
    /// Maximum precipitation probability during the hour (%)
    PrecipitationProbabilityMax,
    /// Maximum wind speed at 10 meters above ground during the hour (km/h)
    WindSpeed10mMax,
    /// Maximum wind gust speed at 10 meters above ground during the hour (km/h)
    WindGusts10mMax,
    /// Dominant wind direction at 10 meters above ground during the hour (°, 0-360)
    WindDirection10mDominant,
    /// Sum of shortwave radiation during the hour (MJ/m²)
    ShortwaveRadiationSum,
    /// Reference evapotranspiration (FAO Penman-Monteith) for the day (mm)
    Et0FaoEvapotranspiration,
}

impl AsRef<str> for DailyVariables {
    fn as_ref(&self) -> &str {
        match self {
            DailyVariables::WeatherCode => "weather_code",
            DailyVariables::Temperature2mMax => "temperature_2m_max",
            DailyVariables::Temperature2mMin => "temperature_2m_min",
            DailyVariables::ApparentTemperatureMax => "apparent_temperature_max",
            DailyVariables::ApparentTemperatureMin => "apparent_temperature_min",
            DailyVariables::Sunrise => "sunrise",
            DailyVariables::Sunset => "sunset",
            DailyVariables::DaylightDuration => "daylight_duration",
            DailyVariables::SunshineDuration => "sunshine_duration",
            DailyVariables::UvIndex => "uv_index",
            DailyVariables::UvIndexClearSky => "uv_index_clear_sky",
            DailyVariables::RainSum => "rain_sum",
            DailyVariables::ShowersSum => "showers_sum",
            DailyVariables::SnowfallSum => "snowfall_sum",
            DailyVariables::PrecipitationSum => "precipitation_sum",
            DailyVariables::PrecipitationHours => "precipitation_hours",
            DailyVariables::PrecipitationProbabilityMax => "precipitation_probability_max",
            DailyVariables::WindSpeed10mMax => "wind_speed_10m_max",
            DailyVariables::WindGusts10mMax => "wind_gusts_10m_max",
            DailyVariables::WindDirection10mDominant => "wind_direction_10m_dominant",
            DailyVariables::ShortwaveRadiationSum => "shortwave_radiation_sum",
            DailyVariables::Et0FaoEvapotranspiration => "et0_fao_evapotranspiration",
        }
    }
}
