use std::collections::HashMap;

use next_web_core::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{
    credential::{GenericAccessToken, GetAccessToken, GetStableAccessToken, StableAccessToken},
    error::Error::InternalServer,
    response::Response,
    Result,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    nickname: String,
    gender: u8,
    country: String,
    province: String,
    city: String,
    avatar: String,
    watermark: Watermark,
}

impl User {
    pub fn nickname(&self) -> &str {
        &self.nickname
    }

    pub fn gender(&self) -> u8 {
        self.gender
    }

    pub fn country(&self) -> &str {
        &self.country
    }

    pub fn province(&self) -> &str {
        &self.province
    }

    pub fn city(&self) -> &str {
        &self.city
    }

    pub fn avatar(&self) -> &str {
        &self.avatar
    }

    pub fn app_id(&self) -> &str {
        &self.watermark.app_id
    }

    pub fn timestamp(&self) -> u64 {
        self.watermark.timestamp
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserBuilder {
    #[serde(rename = "nickName")]
    nickname: String,
    gender: u8,
    country: String,
    province: String,
    city: String,
    #[serde(rename = "avatarUrl")]
    avatar: String,
    watermark: WatermarkBuilder,
}

impl UserBuilder {
    pub(crate) fn build(self) -> User {
        User {
            nickname: self.nickname,
            gender: self.gender,
            country: self.country,
            province: self.province,
            city: self.city,
            avatar: self.avatar,
            watermark: self.watermark.build(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contact {
    phone_number: String,
    pure_phone_number: String,
    country_code: String,
    watermark: Watermark,
}

impl Contact {
    pub fn phone_number(&self) -> &str {
        &self.phone_number
    }

    pub fn pure_phone_number(&self) -> &str {
        &self.pure_phone_number
    }

    pub fn country_code(&self) -> &str {
        &self.country_code
    }

    pub fn app_id(&self) -> &str {
        &self.watermark.app_id
    }

    pub fn timestamp(&self) -> u64 {
        self.watermark.timestamp
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ContactBuilder {
    #[serde(rename = "phone_info")]
    inner: PhoneInner,
}

impl ContactBuilder {
    pub(crate) fn build(self) -> Contact {
        Contact {
            phone_number: self.inner.phone_number,
            pure_phone_number: self.inner.pure_phone_number,
            country_code: self.inner.country_code,
            watermark: self.inner.watermark.build(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PhoneInner {
    #[serde(rename = "phoneNumber")]
    phone_number: String,
    #[serde(rename = "purePhoneNumber")]
    pure_phone_number: String,
    country_code: String,
    watermark: WatermarkBuilder,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Watermark {
    app_id: String,
    timestamp: u64,
}

#[derive(Debug, Deserialize, Clone)]
struct WatermarkBuilder {
    #[serde(rename = "appid")]
    app_id: String,
    timestamp: u64,
}

impl WatermarkBuilder {
    fn build(self) -> Watermark {
        Watermark {
            app_id: self.app_id,
            timestamp: self.timestamp,
        }
    }
}

#[async_trait]
pub trait GetContact {
    const PHONE: &str = "https://api.weixin.qq.com/wxa/business/getuserphonenumber";

    async fn get_contact(&self, code: &str, open_id: Option<&str>) -> Result<Contact>;
}

#[async_trait]
impl GetContact for GenericAccessToken {
    async fn get_contact(&self, code: &str, open_id: Option<&str>) -> Result<Contact> {
        use reqwest::Client;

        event!(Level::DEBUG, "code: {}, open_id: {:?}", code, open_id);

        let mut query = HashMap::new();
        let mut body = HashMap::new();

        query.insert("access_token", self.access_token().await?);
        body.insert("code", code);

        if let Some(open_id) = open_id {
            body.insert("openid", open_id);
        }

        let response = Client::new()
            .post(Self::PHONE)
            .query(&query)
            .json(&body)
            .send()
            .await?;

        event!(Level::DEBUG, "response: {:#?}", response);

        if response.status().is_success() {
            let response = response.json::<Response<ContactBuilder>>().await?;

            let builder = response.extract()?;

            event!(Level::DEBUG, "contact builder: {:#?}", builder);

            Ok(builder.build())
        } else {
            Err(InternalServer(response.text().await?))
        }
    }
}

#[async_trait]
impl GetContact for GenericAccessToken<StableAccessToken> {
    async fn get_contact(&self, code: &str, open_id: Option<&str>) -> Result<Contact> {
        use reqwest::Client;

        event!(Level::DEBUG, "code: {}, open_id: {:?}", code, open_id);

        let mut query = HashMap::new();
        let mut body = HashMap::new();

        query.insert("access_token", self.access_token().await?);
        body.insert("code", code);

        if let Some(open_id) = open_id {
            body.insert("openid", open_id);
        }

        let response = Client::new()
            .post(Self::PHONE)
            .query(&query)
            .json(&body)
            .send()
            .await?;

        event!(Level::DEBUG, "response: {:#?}", response);

        if response.status().is_success() {
            let response = response.json::<Response<ContactBuilder>>().await?;

            let builder = response.extract()?;

            event!(Level::DEBUG, "contact builder: {:#?}", builder);

            Ok(builder.build())
        } else {
            Err(InternalServer(response.text().await?))
        }
    }
}
