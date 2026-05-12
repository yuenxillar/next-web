use std::sync::Arc;

use bytes::Bytes;

use crate::filter::factory::local_response_cache::{
    LocalResponseCacheManager, LocalResponseCacheRequestState, PendingLocalResponseCacheEntry,
};
use pingora::{
    http::{RequestHeader, ResponseHeader},
    Result,
};

#[derive(Clone)]
pub struct RequestContext {
    pub fallback_id: Option<String>,
    pub route_id: Option<String>,
    
    pub original_request_path: Option<String>,
    pub buffer_response_body: bool,
    pub response_body_buffer: Vec<u8>,
    pub local_response_cache_manager: Option<Arc<LocalResponseCacheManager>>,
    pub local_response_cache_request: Option<LocalResponseCacheRequestState>,
    pub pending_local_response_cache: Option<PendingLocalResponseCacheEntry>,
    pub session: Option<String>,
    pub direct_response: Option<DirectResponse>,
}

#[derive(Clone)]
pub struct DirectResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

impl RequestContext {
    pub fn respond_with_text(
        &mut self,
        status: u16,
        headers: Vec<(String, String)>,
        body: impl Into<String>,
    ) -> Result<()> {
        // Store the response on the context and stop the proxy flow with an HTTP status error.
        self.direct_response = Some(DirectResponse {
            status,
            headers,
            body: Bytes::from(body.into()),
        });

        Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(status)))
    }

    pub fn respond_with_empty(
        &mut self,
        status: u16,
        headers: Vec<(String, String)>,
    ) -> Result<()> {
        self.direct_response = Some(DirectResponse {
            status,
            headers,
            body: Bytes::new(),
        });

        Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(status)))
    }

    pub fn respond_with_body(
        &mut self,
        status: u16,
        headers: Vec<(String, String)>,
        body: Bytes,
    ) -> Result<()> {
        self.direct_response = Some(DirectResponse {
            status,
            headers,
            body,
        });

        Err(pingora::Error::new(pingora::ErrorType::HTTPStatus(status)))
    }
}

#[derive(Debug)]
pub enum HeaderAndBody<'a, 'b> {
    Req {
        req_header: Option<&'a mut RequestHeader>,
        req_body: Option<&'b mut Bytes>,
    },

    Resp {
        resp_body: Option<&'b mut Bytes>,
        resp_header: Option<&'a mut ResponseHeader>,
    },

    None,
}

impl<'a, 'b> HeaderAndBody<'a, 'b> {
    pub fn with_req_header(req_header: &'a mut RequestHeader) -> Self {
        Self::Req {
            req_header: Some(req_header),
            req_body: None,
        }
    }

    pub fn with_resp_header(resp_header: &'a mut ResponseHeader) -> Self {
        Self::Resp {
            resp_body: None,
            resp_header: Some(resp_header),
        }
    }

    pub fn with_req_body(req_body: &'b mut Option<Bytes>) -> Self {
        Self::Req {
            req_header: None,
            req_body: req_body.as_mut().map(|body| body),
        }
    }

    pub fn with_resp_body(resp_body: &'b mut Option<Bytes>) -> Self {
        Self::Resp {
            resp_body: resp_body.as_mut().map(|body| body),
            resp_header: None,
        }
    }
}
