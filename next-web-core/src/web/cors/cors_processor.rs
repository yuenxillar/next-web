use crate::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    web::cors::CorsConfiguration,
};

pub trait CorsProcessor
where
    Self: Send + Sync,
{
    fn process_request(
        &self,
        configuration: Option<&CorsConfiguration>,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<bool, BoxError>;
}
