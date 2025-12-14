use std::result::Result;

use async_trait::async_trait;

use crate::BoxError;

#[async_trait]
pub trait WeatherService: Send + Sync {
    type RequestParams: serde::Serialize + Send;

    type WeatherResponse: std::fmt::Debug + serde::de::DeserializeOwned;

    async fn weather_inquiry(
        &self,
        params: Self::RequestParams,
    ) -> Result<Self::WeatherResponse, BoxError>;
}
