use std::collections::HashMap;

use next_web_core::{async_trait, core::service::Service};
use reqwest::Error;

use crate::core::service::SmsSendService;

#[derive(Clone)]
pub struct AliyunSmsService {}

impl Service for AliyunSmsService {}

#[async_trait]
impl SmsSendService for AliyunSmsService {
    type Response = ();

    async fn send_sms<'a>(
        &self,
        phone_numbers: Vec<&'a str>,
        sign_name: &'a str,
        template_code: &'a str,
        template_param: &'a str,
        expand_params: Option<HashMap<&'a str, &'a str>>,
    ) -> Result<Self::Response, Error> {
        let mut params= HashMap::new();

        let phone_numbers = phone_numbers.join(",");
        params.insert("SignName", sign_name);
        params.insert("PhoneNumbers", phone_numbers.as_str());
        params.insert("TemplateCode", template_code);
        params.insert("TemplateParam", template_param);

        expand_params.map(|var| params.extend(var));

        
        Ok(())
    }

    async fn send_batch_sms(&self) {
        unimplemented!()
    }

    fn common_params(&self) -> HashMap<&str, String> {
        let mut params = HashMap::new();

        params.insert("Action", "SendSms".into());
        params
    }
}

mod test {
    use super::*;

    #[test]
    fn test() {
        let params = vec!["178888", "15653123", "165463313"].join(",");
        println!("{:?}", params);
    }
}
