use bytes::Bytes;
use pingora::http::{RequestHeader, ResponseHeader};

use crate::{
    context::{HeaderAndBody, RequestContext},
    server::ServerWebExchange,
};

pub struct DefaultServerWebExchange<'ctx, 'a, 'b> {
    pub(crate) req_ctx: &'ctx mut RequestContext,
    pub(crate) header_and_body: HeaderAndBody<'a, 'b>,
}

impl<'ctx, 'a, 'b> DefaultServerWebExchange<'ctx, 'a, 'b> {
    pub fn new(req_ctx: &'ctx mut RequestContext, header_and_body: HeaderAndBody<'a, 'b>) -> Self {
        Self {
            req_ctx,
            header_and_body,
        }
    }

    pub fn req_ctx(&self) -> &RequestContext {
        self.req_ctx
    }

    pub fn header_and_body(&self) -> &HeaderAndBody<'a, 'b> {
        &self.header_and_body
    }
}

impl<'ctx, 'a, 'b> ServerWebExchange for DefaultServerWebExchange<'ctx, 'a, 'b> {
    fn request_context(&mut self) -> &mut RequestContext {
        self.req_ctx
    }

    fn request_body(&mut self) -> Option<&mut Bytes> {
        match &mut self.header_and_body {
            HeaderAndBody::Req { req_body, .. } => req_body.as_deref_mut(),
            _ => None,
        }
    }

    fn response_body(&mut self) -> Option<&mut Bytes> {
        match &mut self.header_and_body {
            HeaderAndBody::Resp { resp_body, .. } => resp_body.as_deref_mut(),
            _ => None,
        }
    }

    fn request_header(&mut self) -> Option<&mut RequestHeader> {
        match &mut self.header_and_body {
            HeaderAndBody::Req { req_header, .. } => req_header.as_deref_mut(),
            _ => None,
        }
    }

    fn response_header(&mut self) -> Option<&mut ResponseHeader> {
        match &mut self.header_and_body {
            HeaderAndBody::Resp { resp_header, .. } => resp_header.as_deref_mut(),
            _ => None,
        }
    }
}
