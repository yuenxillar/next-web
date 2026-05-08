use crate::{
    AlipayResult,
    client::AlipayClient,
    payment::{common::AlipayTradePayExt, face::model::*, model::AlipayResponse},
};

pub trait FacePay {
    fn initialize(
        &self,
        req: FacePayInitializeRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<FacePayInitializeResponse>>> + Send;

    fn query(
        &self,
        req: FacePayQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<FacePayQueryResponse>>> + Send;
}

impl<T> FacePay for T
where
    T: AsRef<AlipayClient>,
    T: Sync,
{
    fn initialize(
        &self,
        req: FacePayInitializeRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<FacePayInitializeResponse>>> + Send {
        self.call(req)
    }

    fn query(
        &self,
        req: FacePayQueryRequest,
    ) -> impl Future<Output = AlipayResult<AlipayResponse<FacePayQueryResponse>>> + Send {
        self.call(req)
    }
}
