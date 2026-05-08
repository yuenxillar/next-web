use crate::{
    AlipayResult, Method,
    client::AlipayClient,
    payment::{app::model::AlipayAppPayRequest, page::model::AlipayTradePagePayRequest},
};

pub trait TradeAppPay {
    fn app_pay(
        &self,
        req: AlipayAppPayRequest,
    ) -> impl Future<Output = AlipayResult<String>> + Send;
}

impl<T> TradeAppPay for T
where
    T: AsRef<AlipayClient>,
    T: Sync,
{
    fn app_pay(
        &self,
        req: AlipayAppPayRequest,
    ) -> impl Future<Output = AlipayResult<String>> + Send {
        async move {
            let params =
                self.as_ref()
                    .signed_params(AlipayTradePagePayRequest::method(), &req, None)?;

            let url = self.as_ref().config().gateway_url();

            let resp = self
                .as_ref()
                .client()
                .post(url)
                .query(&params)
                .send()
                .await?
                .text()
                .await?;

            Ok(resp)
        }
    }
}
