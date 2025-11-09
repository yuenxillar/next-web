use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::core::session::mgt::session_context::SessionContext;

pub trait WebSessionContext
where
    Self: Send + Sync,
    Self: SessionContext,
{
    fn get_request(&mut self) -> &mut dyn HttpRequest;

    fn get_response(&mut self) -> &mut dyn HttpResponse;
}
