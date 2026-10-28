//! Per-request state shared by the proxy, the filters and the predicates.

mod request_context;

pub(crate) use request_context::{HeaderAndBody, RequestContext};
