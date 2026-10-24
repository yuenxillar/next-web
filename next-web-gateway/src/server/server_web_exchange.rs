use bytes::Bytes;
use pingora::http::{RequestHeader, ResponseHeader};

use crate::context::RequestContext;

pub trait ServerWebExchange: Send {
    fn request_context(&mut self) -> &mut RequestContext;

    fn request_body(&mut self) -> Option<&mut Bytes>;

    fn response_body(&mut self) -> Option<&mut Bytes>;

    fn request_header(&mut self) -> Option<&mut RequestHeader>;

    fn response_header(&mut self) -> Option<&mut ResponseHeader>;
}
