use async_trait::async_trait;

use crate::{
    BoxError,
    open_weather::{
        interface::weather_service::WeatherService,
        region::international::open_weather::{
            model::weather_info::WeatherInfo,
            request_params::weather_query_params::WeatherQueryParams,
            response::weather_response::WeatherResponse, weather_service::OpenWeatherService,
        },
    },
};

#[async_trait]
impl WeatherService for OpenWeatherService {
    type RequestParams = WeatherQueryParams;

    type WeatherResponse = WeatherResponse<WeatherInfo>;
    async fn weather_inquiry(
        &self,
        mut query_params: Self::RequestParams,
    ) -> Result<Self::WeatherResponse, BoxError> {
        let base_url = "https://api.openweathermap.org/data/3.0/onecall";
        query_params.appid = Some(self.appid.clone());
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
        region::international::open_weather::{
            request_params::weather_query_params::WeatherQueryParams,
            weather_service::OpenWeatherService,
        },
    };

    #[tokio::test]
    async fn send_weather_inquiry() {
        let appid = "your_api_key";
        let weather_service = OpenWeatherService::new(appid);
        let query_params = WeatherQueryParams::new(20.000, 120.111);

        let resp = weather_service.weather_inquiry(query_params).await.unwrap();
        println!("resp: {:?}", resp)
    }
}
