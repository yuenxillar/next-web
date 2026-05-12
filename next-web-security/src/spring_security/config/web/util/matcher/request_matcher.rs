use std::fmt::Debug;
use std::str::FromStr;

use axum::extract::Request;
use next_web_core::{util::http_method::HttpMethod, DynClone};

use crate::config::web::util::matcher::ant_path_request_matcher::AntPathRequestMatcher;

pub trait RequestMatcher
where
    Self: DynClone,
    Self: Debug + Send + Sync,
{
    fn matches(&self, request: &Request) -> bool;
}

next_web_core::clone_trait_object!(RequestMatcher);

impl RequestMatcher for HttpMethod {
    fn matches(&self, request: &Request) -> bool {
        HttpMethod::from_str(request.method().as_str())
            .map(|method| method == *self)
            .unwrap_or(false)
    }
}

impl<T> RequestMatcher for (HttpMethod, T)
where
    T: IntoIterator<Item = &'static str>,
    T: Clone + Debug + Send + Sync,
{
    fn matches(&self, request: &Request) -> bool {
        self.1.clone().into_iter().any(|pattern| {
            AntPathRequestMatcher::from((Some(self.0), pattern)).matches(request)
        })
    }
}

impl RequestMatcher for Vec<&'static str> {
    fn matches(&self, request: &Request) -> bool {
        self.iter()
            .any(|pattern| AntPathRequestMatcher::from(*pattern).matches(request))
    }
}
