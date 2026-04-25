use crate::{client::AlipayClient, config::AlipayConfig};

#[derive(Clone)]
pub struct AliPayService {
    client: AlipayClient,
}

impl AliPayService {
    pub fn new(config: AlipayConfig) -> Self {
        let client = AlipayClient::new(config);

        Self { client }
    }
}


impl AsRef<AlipayClient> for AliPayService {
    fn as_ref(&self) -> &AlipayClient {
        &self.client
    }
}