use crate::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

pub trait RequestDispatcher {
    fn forward(&self, req: &dyn HttpRequest, resp: &mut dyn HttpResponse) -> Result<(), BoxError>;

    fn include(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), BoxError>;
}
