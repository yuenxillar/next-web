use std::any::Any;

use async_trait::async_trait;
use dyn_clone::{DynClone, clone_trait_object};

use crate::{
    error::BoxError,
    traits::{
        filter::http_filter_chain::HttpFilterChain,
        http::{http_request::HttpRequest, http_response::HttpResponse},
        nameable::Nameable,
    },
};

#[async_trait]
pub trait HttpFilter
where
    Self: Send + Sync,
    Self: Any + DynClone,
    Self: Nameable,
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
