use reqwest::Client;

#[derive(Debug, Clone)]
pub struct SmsClient {
    client: Client,
    sign_name: String,
}


impl SmsClient  {
    
    pub fn client(&self) -> &Client {
        &self.client
    }


    fn test(&self) {
        // self.client.post("url").send()
        // "sms.tencentcloudapi.com"
        // "dysmsapi.aliyuncs.com"
    }
}