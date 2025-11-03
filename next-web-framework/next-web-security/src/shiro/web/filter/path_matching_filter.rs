use next_web_core::{async_trait, traits::http::{http_request::HttpRequest, http_response::HttpResponse}};

use crate::core::util::{ant_path_matcher::AntPathMatcher, object::Object, pattern_matcher::PatternMatcher};

#[derive(Clone)]
pub struct PathMatchingFilter<T = AntPathMatcher> {
    path_matcher: T,
}

impl<T> PathMatchingFilter<T> where T: PatternMatcher {

    const DEFAULT_PATH_SEPARATOR: &str = "/";

    
    pub fn paths_match(&self, mut path: &str, request: &dyn HttpRequest) -> bool {

        
        let mut req_path = request.path();

        let mut result = self.path_matcher.matches(path, req_path);

        if !result {
            if !req_path.is_empty() && !Self::DEFAULT_PATH_SEPARATOR.eq(req_path) && req_path.ends_with(Self::DEFAULT_PATH_SEPARATOR) {
                req_path = & req_path[0..req_path.len() - 1];
            }

            if !path.is_empty() && !Self::DEFAULT_PATH_SEPARATOR.eq(path) && path.ends_with(Self::DEFAULT_PATH_SEPARATOR) {
                path = & path[0..path.len() - 1];
            }

            result = self.path_matcher.matches(path, req_path);
        }

        result
    }
}

impl<T> Default for PathMatchingFilter<T>
where
    T: PatternMatcher + Default,
{
    fn default() -> Self {
        Self {
            path_matcher: T::default(),
        }
    }
}

#[async_trait]
pub trait PathMatchingFilterExt {
    async fn on_pre_handle(&self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse, mapped_value: &Object)  -> bool;
}