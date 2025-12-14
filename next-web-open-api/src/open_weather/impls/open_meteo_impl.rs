use async_trait::async_trait;

use crate::{
    BoxError,
    open_weather::{
        interface::weather_service::WeatherService,
        region::international::open_meteo::{
            model::weather_info::WeatherInfo,
            request_params::weather_query_params::WeatherQueryParams,
            response::weather_response::WeatherResponse, weather_service::OpenMeteoWeatherService,
        },
    },
};

#[async_trait]
impl WeatherService for OpenMeteoWeatherService {
    type RequestParams = WeatherQueryParams;

    type WeatherResponse = WeatherResponse<WeatherInfo>;
    async fn weather_inquiry(
        &self,
        query_params: Self::RequestParams,
    ) -> Result<Self::WeatherResponse, BoxError> {
        let base_url = "https://api.open-meteo.com/v1/forecast";
        let url = build_url(base_url, query_params)?;
        // query_params.api_key = Some(self.api_key.clone());
        let resp = self.client.get(url).send().await?;

        println!("url: {}", resp.url());
        Ok(resp.json().await?)
    }
}

fn build_url(base_url: &str, params: WeatherQueryParams) -> Result<String, &'static str> {
    let WeatherQueryParams {
        latitude,
        longitude,
        options,
    } = params;

    if latitude < -90.0 || latitude > 90.0 {
        return Err("Latitude must be between -90 and 90");
    }

    if longitude < -180.0 || longitude > 180.0 {
        return Err("Longitude must be between -180 and 180");
    }

    let mut query = String::new();
    for weather_type in options {
        let str = weather_type.to_string();
        query.push('&');
        query.push_str(&str);
    }

    Ok(format!(
        "{}?latitude={}&longitude={}{}",
        base_url,
        latitude,
        longitude,
        &query
    ))
}

#[inline]
pub fn url_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 3); // Pre-allocate space

    for byte in input.bytes() {
        match byte {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char)
            }
            _ => {
                result.push('%');
                result.push(hex_digit(byte >> 4));
                result.push(hex_digit(byte & 0x0F));
            }
        }
    }
    result
}

#[inline]
fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + value - 10) as char,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod weather_service_tests {
    use crate::open_weather::{
        interface::weather_service::WeatherService,
        region::international::open_meteo::{
            request_params::weather_query_params::{
                CurrentVariables, WeatherQueryParams, WeatherType,
            },
            weather_service::OpenMeteoWeatherService,
        },
    };

    #[tokio::test]
    async fn send_weather_inquiry() {
        let api_key = "your_api_key";
        let weather_service = OpenMeteoWeatherService::new(api_key);

        let options = vec![WeatherType::CurrentWeather(vec![
            CurrentVariables::Temperature2m,
            CurrentVariables::RelativeHumidity2m,
            CurrentVariables::ApparentTemperature,
            CurrentVariables::WeatherCode,
            CurrentVariables::CloudCover,
            CurrentVariables::PressureMsl,
        ])];
        let query_params = WeatherQueryParams::new(20.000, 120.111, options);

        let resp = weather_service.weather_inquiry(query_params).await.unwrap();
        println!("resp: {:?}", resp)
    }
}
