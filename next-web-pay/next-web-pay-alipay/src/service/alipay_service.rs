use std::{borrow::Borrow, collections::BTreeMap};

use crate::{client::AlipayClient, config::AlipayConfig, sign::build_notify_sign_content};

#[derive(Clone)]
pub struct AliPayService {
    client: AlipayClient,
}

impl AliPayService {
    pub fn new(config: AlipayConfig) -> Self {
        let client = AlipayClient::new(config);

        Self { client }
    }

    pub fn verify_sign<'a, I, T>(&self, params: &'a I, sign: &str) -> bool
    where
        &'a I: IntoIterator<Item = (&'a T, &'a T)>,
        T: Borrow<str> + 'a,
    {
        let pairs = params
            .into_iter()
            .map(|(k, v)| (k.borrow(), v.borrow()))
            .filter(|(k, _)| *k != "sign" && *k != "sign_type")
            .collect::<BTreeMap<&str, &str>>();
        let content = build_notify_sign_content(&pairs);

        self.client
            .verify_signature(content.as_str(), sign)
            .inspect_err(|e| tracing::error!("verify sign[{}] failed: {}", sign, e))
            .ok()
            .unwrap_or_default()
    }
}

impl AsRef<AlipayClient> for AliPayService {
    fn as_ref(&self) -> &AlipayClient {
        &self.client
    }
}
