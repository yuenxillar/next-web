use std::collections::BTreeMap;

use next_web_core::{async_trait, core::service::Service, error::BoxError};
use reqwest::Client;

use crate::core::client::SmsClient;

#[derive(Clone)]
pub struct SmsService<T: SmsSendService> {
    client: Client,
    send_service: T,
}


#[async_trait]
pub trait SmsSendService: Service {

    type Response: serde::de::DeserializeOwned;

    async fn send_sms<'a>(
        &self,
        phone_number: &'a str,
        sign_name: &'a str,
        template_code: &'a str,
        template_param: &'a str,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> Result<Self::Response, BoxError>;

    async fn send_batch_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_names: Vec<&'a str>,
        template_code: &'a str,
        template_param: Vec<&'a str>,
        expand_params: Option<BTreeMap<&'a str, String>>,
    ) -> Result<Self::Response, BoxError>;

    fn check_validity<'a>(&self, phone_number: &'a str, sign_name: &'a str) -> bool;

    fn url(&self) -> &str;

    fn common_req_headers(&self) -> BTreeMap<&str, String>;
}