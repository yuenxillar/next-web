use std::borrow::Cow;
use std::collections::HashMap;

use next_web_core::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::{client::Client, response::Response};

#[async_trait]
pub trait TemplateMessageInterface {
    const GET_TEMPLATE_LIST_URL: &'static str =
        "https://api.weixin.qq.com/cgi-bin/template/get_all_private_template?access_token=";

    async fn get_template_list(&self, access_token: &str) -> Result<Response<TemplateMessageList>>;

    const DELETE_TEMPLATE_URL: &'static str =
        "https://api.weixin.qq.com/cgi-bin/template/del_private_template?access_token=";

    async fn delete_template(
        &self,
        access_token: &str,
        template_id: String,
    ) -> Result<Response<()>>;

    const SEND_TEMPLATE_URL: &'static str =
        "https://api.weixin.qq.com/cgi-bin/message/template/send?access_token=";

    async fn send_template_message(
        &self,
        access_token: &str,
        data: SendTemplateMessageRequest,
    ) -> Result<Response<()>>;
}

#[derive(Clone, Deserialize)]
pub struct TemplateMessageList {
    pub template_list: Vec<TemplateMessageData>,
}

#[derive(Clone, Deserialize)]
pub struct TemplateMessageData {
    pub template_id: Cow<'static, str>,
    pub title: Cow<'static, str>,
    pub primary_industry: Cow<'static, str>,
    pub deputy_industry: Cow<'static, str>,
    pub content: Cow<'static, str>,
    pub example: Cow<'static, str>,
}

#[derive(Clone, Serialize)]
pub struct SendTemplateMessageRequest {
    pub touser: String,
    pub template_id: String,
    pub url: String,
    pub miniprogram: Miniprogram,
    pub client_msg_id: String,
    pub data: HashMap<String, String>,
}

#[derive(Clone, Serialize)]
pub struct Miniprogram {
    pub appid: String,
    pub pagepath: String,
}

#[async_trait]
impl TemplateMessageInterface for Client {
    async fn get_template_list(&self, access_token: &str) -> Result<Response<TemplateMessageList>> {
        let result = self
            .request()
            .get(format!("{}{}", Self::GET_TEMPLATE_LIST_URL, access_token))
            .send()
            .await?;
        Ok(result.json().await?)
    }

    async fn delete_template(
        &self,
        access_token: &str,
        template_id: String,
    ) -> Result<Response<()>> {
        let result = self
            .request()
            .post(format!("{}{}", Self::DELETE_TEMPLATE_URL, access_token))
            .body(format!("{{\"template_id\": \"{}\"}}", template_id))
            .send()
            .await?;
        Ok(result.json().await?)
    }

    async fn send_template_message(
        &self,
        access_token: &str,
        data: SendTemplateMessageRequest,
    ) -> Result<Response<()>> {
        let result = self
            .request()
            .post(format!("{}{}", Self::DELETE_TEMPLATE_URL, access_token))
            .body(serde_json::to_string(&data)?)
            .send()
            .await?;
        Ok(result.json().await?)
    }
}
