use next_web_core::async_trait;

use crate::client::Client;
use crate::response::Response;
use crate::Result;

use super::model::CreateMenuRequest;

#[async_trait]
pub trait CustomizeMenuInterface {
    const CREATE_URL: &'static str = "https://api.weixin.qq.com/cgi-bin/menu/create?access_token=";

    async fn create(&self, access_token: &str, request: CreateMenuRequest) -> Result<Response<()>>;

    const QUERY_URL: &'static str =
        "https://api.weixin.qq.com/cgi-bin/get_current_selfmenu_info?access_token=";

    async fn query(&self, access_token: &str) -> Result<Response<()>> ; 

    const DELETE_URL: &'static str = "https://api.weixin.qq.com/cgi-bin/menu/delete?access_token=";

    async fn delete(&self, access_token: &str) -> Result<Response<()>>;
}

#[async_trait]
impl CustomizeMenuInterface for Client {
    async fn create(&self, access_token: &str, request: CreateMenuRequest) -> Result<Response<()>> {
        let result = self
            .request()
            .post(format!("{}{}", Self::CREATE_URL, access_token))
            .json(&request)
            .send()
            .await?;
        Ok(result.json().await?)
    }

    async fn query(&self, access_token: &str) -> Result<Response<()>> {
        let result = self
            .request()
            .get(format!("{}{}", Self::QUERY_URL, access_token))
            .send()
            .await?;
        Ok(result.json().await?)
    }

    async fn delete(&self, access_token: &str)  -> Result<Response<()>> {
        let result = self
            .request()
            .get(format!("{}{}", Self::DELETE_URL, access_token))
            .send()
            .await?;
        Ok(result.json().await?)
    }
}
