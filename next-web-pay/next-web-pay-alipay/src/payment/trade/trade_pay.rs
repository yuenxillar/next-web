use crate::{
    AlipayError, AlipayResult,
    client::AlipayClient,
    payment::{
        model::AlipayResponse,
        trade::model::{
            TradePayRequest, TradePayResponse, TradePrecreateRequest, TradePrecreateResponse,
        },
    },
};

pub trait TradePay {
    fn precreate(
        &self,
        req: TradePrecreateRequest,
    ) -> impl Future<Output = AlipayResult<TradePrecreateResponse>> + Send;

    fn pay(
        &self,
        req: TradePayRequest,
    ) -> impl Future<Output = AlipayResult<TradePayResponse>> + Send;
}

impl<T> TradePay for T
where
    T: AsRef<AlipayClient>,
    T: Sync,
{
    fn precreate(
        &self,
        req: TradePrecreateRequest,
    ) -> impl Future<Output = AlipayResult<TradePrecreateResponse>> + Send {
        async move {
            let method = "alipay.trade.precreate";

            let query = self
                .as_ref()
                .signed_params(method, &req)
                .map_err(|err| AlipayError::Signing(err))?;

            let mut form_data = Vec::with_capacity(2);
            form_data.push(("biz_content", serde_json::to_string(&req)?));
            if let Some(app_auth_token) = self.as_ref().config().app_auth_token() {
                form_data.push(("app_auth_token", app_auth_token.into()));
            }

            let url = self.as_ref().config().gateway_url();
            let resp = self
                .as_ref()
                .client()
                .post(url)
                .query(&query)
                .form(&form_data)
                .send()
                .await?
                .json::<AlipayResponse<TradePrecreateResponse>>()
                .await?;

            Ok(resp)
        }
    }

    fn pay(
        &self,
        req: TradePayRequest,
    ) -> impl Future<Output = AlipayResult<TradePayResponse>> + Send {
        async move {
            let method = "alipay.trade.pay";

            let query = self
                .as_ref()
                .signed_params(method, &req)
                .map_err(|err| AlipayError::Signing(err))?;

            let mut form_data = Vec::with_capacity(2);
            form_data.push(("biz_content", serde_json::to_string(&req)?));
            if let Some(app_auth_token) = self.as_ref().config().app_auth_token() {
                form_data.push(("app_auth_token", app_auth_token.into()));
            }

            let url = self.as_ref().config().gateway_url();
            let resp = self
                .as_ref()
                .client()
                .post(url)
                .query(&query)
                .form(&form_data)
                .send()
                .await?
                .json::<AlipayResponse<TradePayResponse>>()
                .await?;

            Ok(resp)
        }
    }
}
