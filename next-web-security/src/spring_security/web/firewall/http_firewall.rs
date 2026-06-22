use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::firewall::{
    request_rejected_error::RequestRejectedError, strict_http_firewall::StrictFirewalledRequest,
};

pub trait HttpFirewall
where
    Self: Send + Sync,
{
    /// Provides the request object which will be passed through the filter chain.
    /// RequestRejectedError if the request should be rejected immediately
    ///
    fn get_firewalled_request(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Result<StrictFirewalledRequest, RequestRejectedError>;

    /// Provides the response which will be passed through the filter chain.
    /// esponse the original response
    /// return either the original response or a replacement/wrapper.
    ///
    fn get_firewalled_response<'a>(
        &self,
        response: &'a mut dyn HttpResponse,
    ) -> Box<&'a mut dyn HttpResponse>;
}
