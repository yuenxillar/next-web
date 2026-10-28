use std::collections::BTreeMap;

use crate::{
    AlipayResult, Method, client::AlipayClient, payment::page::model::AlipayTradePagePayRequest,
};

pub trait TradePagePay {
    fn page_pay(
        &self,
        req: AlipayTradePagePayRequest,
    ) -> impl Future<Output = AlipayResult<String>> + Send;
}

impl<T> TradePagePay for T
where
    T: AsRef<AlipayClient>,
    T: Sync,
{
    fn page_pay(
        &self,
        mut req: AlipayTradePagePayRequest,
    ) -> impl Future<Output = AlipayResult<String>> + Send {
        async move {
            let extra_params = req
                .return_url
                .take()
                .map(|return_url| BTreeMap::from([("return_url", return_url)]));
            let query = self.as_ref().signed_params(
                AlipayTradePagePayRequest::method(),
                &req,
                extra_params,
            )?;

            let url = self.as_ref().config().gateway_url();

            let resp = self
                .as_ref()
                .client()
                .post(url)
                .query(&query)
                .send()
                .await?
                .text()
                .await?;

            Ok(resp)
        }
    }
}
