use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};

use next_web_core::{async_trait, core::service::Service};
use reqwest::Error;

use crate::core::service::SmsSendService;

#[derive(Clone)]
pub struct TencentCloudSmsService {}

impl Service for TencentCloudSmsService {}

#[async_trait]
impl SmsSendService for TencentCloudSmsService {
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

        params.insert("X-TC-Action", "DescribeInstances".into());
        let unix_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        params.insert("X-TC-Timestamp", unix_timestamp);
        params.insert("X-TC-Version", "2017-03-12".into());
        params.insert("Authorization", "".into());

        params
    }
}

mod test {

    use std::{collections::BTreeMap, time::Instant};

    use crate::tencent::signature::v3::TencentSignerV3;

    use super::*;

    #[test]
    fn test() {
        let time = Instant::now();
        let signer = TencentSignerV3::new(
            "AKID",
            "SECRET",
            "cvm",
            "ap-guangzhou",
        );
        
        let mut headers = BTreeMap::new();
        headers.insert("X-TC-Version", "2017-03-12".to_string());
        headers.insert("X-TC-Action", "DescribeInstances".to_string());
        headers.insert("Content-Type", "application/json".to_string());
        
        let canonical = signer.sign(
            "POST",
            "/",
            "tencentcloudapi.com",
            &BTreeMap::new(),
            &headers,
            b"test666",
        );

        println!("{:?}", time.elapsed());
        println!("{:?}", canonical);
    }
}
