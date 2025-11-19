use next_web_core::traits::{
    filter::http_filter_chain::HttpFilterChain,
    http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::util::{ant_path_matcher::AntPathMatcher, pattern_matcher::PatternMatcher},
    web::filter::mgt::{
        default_filter_chain_manager::DefaultFilterChainManager,
        filter_chain_manager::FilterChainManager,
    },
};
use std::sync::Arc;

#[derive(Clone)]
pub struct PathMatchingFilterChainResolver {
    filter_chain_manager: Arc<dyn FilterChainManager>,
    pattern_matcher: Arc<dyn PatternMatcher>,
}

impl PathMatchingFilterChainResolver {
    const DEFAULT_PATH_SEPARATOR: &str = "/";

    pub fn get_chain(
        &self,
        request: &dyn HttpRequest,
        _response: &dyn HttpResponse,
        original_chain: &dyn HttpFilterChain,
    ) -> Option<Box<dyn HttpFilterChain>> {
        let filter_chain_manager = self.get_filter_chain_manager();
        if !filter_chain_manager.has_chains() {
            return None;
        }

        let request_uri = self.get_path(request);
        let request_urino_trailing_slash = self.remove_trailing_slash(request_uri);

        // the 'chain names' in this implementation are actually path patterns defined by the user.  We just use them
        // as the chain name for the FilterChainManager's requirements
        for mut path_pattern in filter_chain_manager.get_chain_names() {
            if self.path_matches(path_pattern, request_uri) {
                return Some(filter_chain_manager.proxy(original_chain, path_pattern));
            } else {
                // in spring web, the requestURI "/resource/menus" ---- "resource/menus/" both can access the resource
                // but the pathPattern match "/resource/menus" can not match "resource/menus/"
                // user can use requestURI + "/" to simply bypassed chain filter, to bypassed shiro protect

                path_pattern = self.remove_trailing_slash(path_pattern);
                if self.path_matches(path_pattern, &request_urino_trailing_slash) {
                    return Some(filter_chain_manager.proxy(original_chain, path_pattern));
                }
            }
        }

        None
    }

    pub fn path_matches(&self, pattern: &str, path: &str) -> bool {
        self.pattern_matcher.matches(pattern, path)
    }

    pub fn get_path(&self, request: &dyn HttpRequest) -> &str {
        ""
    }

    pub fn remove_trailing_slash<'a>(&'a self, path: &'a str) -> &'a str {
        if !path.is_empty()
            && !Self::DEFAULT_PATH_SEPARATOR.eq(path)
            && path.ends_with(Self::DEFAULT_PATH_SEPARATOR)
        {
            return &path[0..path.len() - 1];
        }

        path
    }
}

impl PathMatchingFilterChainResolver {
    pub fn get_path_matcher(&self) -> &dyn PatternMatcher {
        self.pattern_matcher.as_ref()
    }

    pub fn set_path_matcher<T: PatternMatcher + 'static>(&mut self, path_matcher: T) {
        self.pattern_matcher = Arc::new(path_matcher);
    }

    pub fn get_filter_chain_manager(&self) -> &dyn FilterChainManager {
        self.filter_chain_manager.as_ref()
    }

    pub fn set_filter_chain_manager<T: FilterChainManager + 'static>(&mut self, manager: T) {
        self.filter_chain_manager = Arc::new(manager);
    }
}

impl Default for PathMatchingFilterChainResolver {
    fn default() -> Self {
        Self {
            filter_chain_manager: Arc::new(DefaultFilterChainManager::default()),
            pattern_matcher: Arc::new(AntPathMatcher::default()),
        }
    }
}
