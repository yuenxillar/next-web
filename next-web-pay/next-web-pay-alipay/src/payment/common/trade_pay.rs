use std::any::Any;

use crate::{
    AlipayError, AlipayResult, Method, Named,
    client::AlipayClient,
    payment::{common::model::*, model::AlipayResponse},
};

pub trait AlipayTradePay {
    fn precreate(
        &self,
        req: AlipayTradePrecreateRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradePrecreateResponse>>> + Send;

    fn pay(
        &self,
        req: AlipayTradePayRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradePayResponse>>> + Send;

    fn query(
        &self,
        req: AlipayTradeQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeQueryResponse>>> + Send;

    fn cancel(
        &self,
        req: AlipayTradeCancelRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeCancelResponse>>> + Send;

    fn refund(
        &self,
        req: AlipayTradeRefundRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeRefundResponse>>> + Send;

    fn refund_query(
        &self,
        req: AlipayTradeRefundQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeRefundQueryResponse>>> + Send;

    fn close(
        &self,
        req: AlipayTradeCloseRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeCloseResponse>>> + Send;
}

impl<T> AlipayTradePay for T
where
    T: AsRef<AlipayClient>,
    T: Sync,
{
    fn precreate(
        &self,
        req: AlipayTradePrecreateRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradePrecreateResponse>>> + Send
    {
        self.call(req)
    }

    fn pay(
        &self,
        req: AlipayTradePayRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradePayResponse>>> + Send {
        self.call(req)
    }

    fn query(
        &self,
        req: AlipayTradeQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeQueryResponse>>> + Send {
        self.call(req)
    }

    fn cancel(
        &self,
        req: AlipayTradeCancelRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeCancelResponse>>> + Send {
        self.call(req)
    }

    fn refund(
        &self,
        req: AlipayTradeRefundRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeRefundResponse>>> + Send {
        self.call(req)
    }

    fn refund_query(
        &self,
        req: AlipayTradeRefundQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeRefundQueryResponse>>> + Send
    {
        self.call(req)
    }

    fn close(
        &self,
        req: AlipayTradeCloseRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<AlipayTradeCloseResponse>>> + Send {
        self.call(req)
    }
}

#[allow(unused)]
trait AlipayTradePayExt {
    fn call<Req, Resp>(
        &self,
        req: Req,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<Resp>>> + Send
    where
        Req: Method + serde::Serialize,
        Req: Send + Sync,
        Req: Any,
        Resp: Named + serde::de::DeserializeOwned,
        Resp: Send + Sync;
}

impl<T> AlipayTradePayExt for T
where
    T: AsRef<AlipayClient>,
    T: Sync,
{
    fn call<Req, Resp>(
        &self,
        req: Req,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<Resp>>> + Send
    where
        Req: Method + serde::Serialize,
        Req: Send + Sync,
        Req: Any,
        Resp: Named + serde::de::DeserializeOwned,
        Resp: Send + Sync,
    {
        async move {
            let query = self
                .as_ref()
                .signed_params(Req::method(), &req, None)
                .map_err(|err| AlipayError::Signing(err))?;

            let mut form_data = Vec::with_capacity(2);
            form_data.push((
                "biz_content",
                query
                    .get("biz_content")
                    .map(ToString::to_string)
                    .unwrap_or(serde_json::to_string(&req)?),
            ));
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
                .json::<AlipayResponse<Resp>>()
                .await?;

            Ok(resp)
        }
    }
}
