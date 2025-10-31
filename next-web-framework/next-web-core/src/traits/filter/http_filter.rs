use std::any::Any;

use async_trait::async_trait;
use dyn_clone::{DynClone, clone_trait_object};

use crate::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

#[async_trait]
pub trait HttpFilter
where
    Self: Send + Sync,
    Self: Any + DynClone
{
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), String>;

    fn supports(&self, name: &str) -> bool { false }
}


clone_trait_object!(HttpFilter);