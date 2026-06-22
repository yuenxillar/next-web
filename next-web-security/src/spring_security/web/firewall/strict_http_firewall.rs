use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::firewall::request_rejected_error::RequestRejectedError;

use super::http_firewall::HttpFirewall;

#[derive(Clone)]
pub struct StrictHttpFirewall {}

impl Default for StrictHttpFirewall {
    fn default() -> Self {
        Self {}
    }
}

impl StrictHttpFirewall {}

impl HttpFirewall for StrictHttpFirewall {
    fn get_firewalled_request(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Result<StrictFirewalledRequest, RequestRejectedError> {
        todo!()
    }

    /// Provides the response which will be passed through the filter chain.
    /// esponse the original response
    /// return either the original response or a replacement/wrapper.
    ///
    fn get_firewalled_response<'a>(
        &self,
        response: &'a mut dyn HttpResponse,
    ) -> Box<&'a mut dyn HttpResponse> {
        todo!()
    }
}

pub struct StrictFirewalledRequest {}
