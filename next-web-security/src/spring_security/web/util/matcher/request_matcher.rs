use std::fmt::Debug;

use indexmap::IndexMap;
use next_web_core::{traits::http::http_request::HttpRequest, util::http_method::HttpMethod};

use crate::web::util::matcher::ant_path_request_matcher::AntPathRequestMatcher;

pub trait RequestMatcher
where
    Self: Debug,
    Self: Send + Sync,
{
    fn matches(&self, request: &dyn HttpRequest) -> bool;

    fn matcher(&self, request: &dyn HttpRequest) -> MatchResult {
        let var_match = self.matches(request);
        MatchResult::new(var_match, Default::default())
    }
}

pub struct MatchResult {
    is_match: bool,
    variables: Option<IndexMap<String, String>>,
}

impl MatchResult {
    fn new(is_match: bool, variables: Option<IndexMap<String, String>>) -> Self {
        Self {
            is_match,
            variables,
        }
    }

    pub fn is_match(&self) -> bool {
        self.is_match
    }

    pub fn get_variables(&self) -> Option<&IndexMap<String, String>> {
        self.variables.as_ref()
    }

    pub fn get_own_variables(self) -> Option<IndexMap<String, String>> {
        self.variables
    }

    pub fn match_with_default() -> Self {
        Self {
            is_match: true,
            variables: Default::default(),
        }
    }

    pub fn match_with_variables(variables: IndexMap<String, String>) -> Self {
        Self {
            is_match: true,
            variables: Some(variables),
        }
    }

    pub fn not_match() -> Self {
        Self {
            is_match: false,
            variables: Default::default(),
        }
    }
}

impl RequestMatcher for HttpMethod {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        &request.method() == self
    }
}

impl<T> RequestMatcher for (HttpMethod, T)
where
    T: IntoIterator<Item = &'static str>,
    T: Clone + Debug + Send + Sync,
{
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        self.1
            .clone()
            .into_iter()
            .any(|pattern| AntPathRequestMatcher::from((Some(self.0), pattern)).matches(request))
    }
}

impl RequestMatcher for Vec<&'static str> {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        self.iter()
            .any(|pattern| AntPathRequestMatcher::from(*pattern).matches(request))
    }
}
