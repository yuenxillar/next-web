use std::any::Any;

use async_trait::async_trait;
use dyn_clone::{clone_trait_object, DynClone};

use crate::traits::{
    filter::http_filter_chain::HttpFilterChain,
    http::{http_request::HttpRequest, http_response::HttpResponse},
};

#[async_trait]
pub trait HttpFilter
where
    Self: Send + Sync,
    Self: Any + DynClone,
{
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), String>;

    #[allow(unused_variables)]
    fn supports(&self, name: &str) -> bool {
        false
    }
}

clone_trait_object!(HttpFilter);
