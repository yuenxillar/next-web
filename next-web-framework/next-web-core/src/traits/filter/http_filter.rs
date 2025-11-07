use std::any::Any;

use async_trait::async_trait;
use dyn_clone::{clone_trait_object, DynClone};

use crate::{
    error::BoxError,
    traits::{
        filter::http_filter_chain::HttpFilterChain,
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

#[async_trait]
pub trait HttpFilter
where
    Self: Send + Sync,
    Self: Any + DynClone,
    Self: Named,
{
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError>;

    #[allow(unused_variables)]
    fn supports(&self, name: &str) -> bool {
        false
    }
}

clone_trait_object!(HttpFilter);
