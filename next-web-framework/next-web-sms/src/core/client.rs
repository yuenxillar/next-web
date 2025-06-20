use reqwest::Client;

#[derive(Clone)]
pub struct SmsClient {
    client: Client,
}

impl SmsClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}
