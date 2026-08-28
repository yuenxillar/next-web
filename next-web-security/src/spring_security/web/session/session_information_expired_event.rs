use next_web_context::EventAttributes;
use next_web_core::{
    http::{HttpFilterChainShare, HttpRequestShare, HttpResponseShare},
    traits::{
        filter::HttpFilterChain,
        http::{http_request::HttpRequest, http_response::HttpResponse},
    },
};

use crate::core::session::SessionInformation;

/// An event that is fired when a `SessionInformation` is detected to have expired.
pub struct SessionInformationExpiredEvent {
    request: HttpRequestShare,
    response: HttpResponseShare,
    filter_chain: Option<HttpFilterChainShare>,

    base: EventAttributes<SessionInformation>,
}

impl SessionInformationExpiredEvent {
    pub fn new(
        session_information: SessionInformation,
        request: HttpRequestShare,
        response: HttpResponseShare,
        filter_chain: Option<HttpFilterChainShare>,
    ) -> Self {
        Self {
            request,
            response,
            filter_chain,

            base: EventAttributes::new(session_information),
        }
    }
}

impl SessionInformationExpiredEvent {
    pub fn request(&mut self) -> &mut dyn HttpRequest {
        &mut self.request
    }

    pub fn response(&mut self) -> &mut dyn HttpResponse {
        &mut self.response
    }

    pub fn filter_chain(&self) -> Option<&dyn HttpFilterChain> {
        self.filter_chain
            .as_ref()
            .map(|s| s as &dyn HttpFilterChain)
    }

    pub fn parts_mut(&mut self) -> (&mut dyn HttpRequest, &mut dyn HttpResponse) {
        (&mut self.request, &mut self.response)
    }

    pub fn session_information(&self) -> &SessionInformation {
        self.base.value()
    }
}
