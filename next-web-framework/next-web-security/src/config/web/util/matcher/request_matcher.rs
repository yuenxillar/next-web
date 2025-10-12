use std::fmt::Debug;

use axum::extract::Request;
use next_web_core::{util::http_method::HttpMethod, DynClone};

pub trait RequestMatcher
where
    Self: DynClone,
    Self: Debug + Send + Sync
{
    fn matches(&self, request: &Request) -> bool;
}

next_web_core::clone_trait_object!(RequestMatcher);

impl RequestMatcher for HttpMethod {
    fn matches(&self, request: &Request) -> bool {
        todo!()
    }
}

impl<T> RequestMatcher for (HttpMethod, T)
where
    T: IntoIterator<Item = &'static str>,
    T: Clone + Debug + Send + Sync
{
    fn matches(&self, request: &Request) -> bool {
        todo!()
    }
}


impl RequestMatcher for Vec<&'static str>
{
    fn matches(&self, request: &Request) -> bool {
        todo!()
    }
}