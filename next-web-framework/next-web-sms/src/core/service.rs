use std::collections::HashMap;

use next_web_core::{async_trait, core::service::Service};
use reqwest::Client;

#[derive(Clone)]
pub struct SmsService<T: SmsSendService> {
    client: Client,
    send_service: T,
}


#[async_trait]
pub trait SmsSendService: Service {
    type Response;

    async fn send_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_name: &'a str,
        template_code: &'a str,
        template_param: &'a str,
        expand_params: Option<HashMap<&'a str, &'a str>>,
    ) -> Result<Self::Response, reqwest::Error>;

    async fn send_batch_sms(&self);


    fn common_params(&self) -> HashMap<&str, String>;
}