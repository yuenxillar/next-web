use async_trait::async_trait;

use crate::{
    BoxError,
    open_weather::{
        interface::weather_service::WeatherService,
        region::domestic::gaode_weather::{
            model::weather_info::WeatherInfo,
            request_params::weather_query_params::WeatherQueryParams,
            response::weater_response::WeatherResponse, weather_service::GaodeWeatherService,
        },
    },
};

#[async_trait]
impl WeatherService for GaodeWeatherService {
    type RequestParams = WeatherQueryParams;

    type WeatherResponse = WeatherResponse<WeatherInfo>;
    async fn weather_inquiry(
        &self,
        mut query_params: Self::RequestParams,
    ) -> Result<Self::WeatherResponse, BoxError> {
        let base_url = "https://restapi.amap.com/v3/weather/weatherInfo";
        query_params.key = Some(self.key.clone());
        let resp = self
            .client
            .get(base_url)
            .query(&query_params)
            .send()
            .await?;

        Ok(resp.json().await?)
    }
}

#[cfg(test)]
mod weather_service_tests {
    use crate::open_weather::{
        interface::weather_service::WeatherService,
        region::domestic::gaode_weather::{
            request_params::weather_query_params::WeatherQueryParams,
            weather_service::GaodeWeatherService,
        },
    };

    #[tokio::test]
    async fn send_weather_inquiry() {
        let key = "your_key";
        let weather_service = GaodeWeatherService::new(key);
        let query_params = WeatherQueryParams::new(110101);
        let resp = weather_service.weather_inquiry(query_params).await.unwrap();
        println!("resp: {:?}", resp)
    }
}
