use std::any::Any;

use async_trait::async_trait;
use dyn_clone::DynClone;

use crate::{
    filter::FilterError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

#[async_trait]
pub trait HttpFilterChain
where
    Self: Send + Sync,
    Self: Any + DynClone,
{
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError>;
}

dyn_clone::clone_trait_object!(HttpFilterChain);
