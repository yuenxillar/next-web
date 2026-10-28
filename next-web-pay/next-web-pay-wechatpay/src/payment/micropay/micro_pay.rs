use std::future::Future;

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};

use crate::{
    Path, ToPath, WechatPayResult,
    client::{V3SignedRequest, WechatPayClient},
    error::WechatPayError,
    payment::{micropay::model::*, model::WechatPayResponse},
};

/// https://pay.weixin.qq.com/doc/global/v3/zh/4013010527
pub trait WechatPayMicroPay {
    fn pay(
        &self,
        req: WechatPayMicroPayRequest,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<WechatPayMicroPayResponse>>> + Send;

    fn query_order(
        &self,
        req: WechatPayQueryOrderRequest,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<WechatPayQueryOrderResponse>>> + Send;

    fn refund(
        &self,
        req: WechatPayRefundRequest,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<WechatPayRefundResponse>>> + Send;
}

impl<T> WechatPayMicroPay for T
where
    T: AsRef<WechatPayClient>,
    T: Sync,
{
    fn pay(
        &self,
        req: WechatPayMicroPayRequest,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<WechatPayMicroPayResponse>>> + Send
    {
        self.do_post(req)
    }

    fn query_order(
        &self,
        req: WechatPayQueryOrderRequest,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<WechatPayQueryOrderResponse>>> + Send
    {
        self.do_get(req)
    }

    fn refund(
        &self,
        req: WechatPayRefundRequest,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<WechatPayRefundResponse>>> + Send
    {
        self.do_post(req)
    }
}

#[allow(unused)]
trait WechatPayMicroPayExt {
    fn do_post<Req, Resp>(
        &self,
        req: Req,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<Resp>>> + Send
    where
        Req: Path,
        Req: serde::Serialize,
        Req: Send + Sync,
        Resp: serde::de::DeserializeOwned,
        Resp: Send + Sync;

    fn do_get<Req, Resp>(
        &self,
        req: Req,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<Resp>>> + Send
    where
        Req: ToPath,
        Req: serde::Serialize,
        Req: Send + Sync,
        Resp: serde::de::DeserializeOwned,
        Resp: Send + Sync;
}

impl<T> WechatPayMicroPayExt for T
where
    T: AsRef<WechatPayClient>,
    T: Sync,
{
    fn do_post<Req, Resp>(
        &self,
        req: Req,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<Resp>>> + Send
    where
        Req: Path,
        Req: serde::Serialize,
        Req: Send + Sync,
        Resp: serde::de::DeserializeOwned,
        Resp: Send + Sync,
    {
        async move {
            let signed = self.as_ref().signed_params("POST", &req, Req::path())?;

            let url = format!("{}{}", self.as_ref().config().base_url(), Req::path());

            let headers = common_headers(&signed)?;

            let resp = self
                .as_ref()
                .client()
                .post(url)
                .body(signed.body)
                .headers(headers)
                .send()
                .await?
                .json::<WechatPayResponse<Resp>>()
                .await?;

            Ok(resp)
        }
    }

    fn do_get<Req, Resp>(
        &self,
        req: Req,
    ) -> impl Future<Output = WechatPayResult<WechatPayResponse<Resp>>> + Send
    where
        Req: ToPath,
        Req: serde::Serialize,
        Req: Send + Sync,
        Resp: serde::de::DeserializeOwned,
        Resp: Send + Sync,
    {
        async move {
            let path = req.to_path()?;
            let signed = self.as_ref().signed_params("GET", &req, &path)?;
            let headers = common_headers(&signed)?;

            let url = format!("{}{}", self.as_ref().config().base_url(), &path);
            let resp = self
                .as_ref()
                .client()
                .get(url)
                .headers(headers)
                .send()
                .await?
                .json::<WechatPayResponse<Resp>>()
                .await?;

            Ok(resp)
        }
    }
}

#[inline]
fn common_headers(signed: &V3SignedRequest) -> Result<HeaderMap, WechatPayError> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(signed.authorization.as_str())
            .map_err(|err| WechatPayError::Custom(err.to_string()))?,
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("WechatPay-Rust"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    Ok(headers)
}
