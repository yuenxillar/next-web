use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::trace;

use crate::{
    core::util::{
        ant_path_matcher::AntPathMatcher, object::Object, pattern_matcher::PatternMatcher,
    },
    web::filter::{
        access_control_filter::AccessControlFilterExt,
        advice_filter::{AdviceFilter, AdviceFilterExt},
    },
};

#[derive(Clone)]
pub struct PathMatchingFilter<T = AntPathMatcher> {
    path_matcher: T,
    pub(crate) applied_paths: IndexMap<String, Object>,
    pub(crate) advice_filter: AdviceFilter,
}

impl<T> PathMatchingFilter<T> {
    pub fn process_path_config(&mut self, path: &str, config: &str) {
        if !config.is_empty() {
            let values = config
                .split(',')
                .map(ToString::to_string)
                .collect::<Vec<String>>();
            self.applied_paths
                .insert(path.to_string(), Object::ListStr(values));
        }
    }
}
impl<T> PathMatchingFilter<T>
where
    T: PatternMatcher,
{
    const DEFAULT_PATH_SEPARATOR: &str = "/";

    pub fn paths_match(&self, mut path: &str, request: &dyn HttpRequest) -> bool {
        let mut req_path = request.path();

        let mut result = self.path_matcher.matches(path, req_path);

        if !result {
            if !req_path.is_empty()
                && !Self::DEFAULT_PATH_SEPARATOR.eq(req_path)
                && req_path.ends_with(Self::DEFAULT_PATH_SEPARATOR)
            {
                req_path = &req_path[0..req_path.len() - 1];
            }

            if !path.is_empty()
                && !Self::DEFAULT_PATH_SEPARATOR.eq(path)
                && path.ends_with(Self::DEFAULT_PATH_SEPARATOR)
            {
                path = &path[0..path.len() - 1];
            }

            result = self.path_matcher.matches(path, req_path);
        }

        result
    }

    #[allow(unused_variables)]
    pub fn after_completion(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: BoxError,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    async fn is_filter_chain_continued(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        path: &str,
        path_config: Option<Object>,
        path_matching_filter_ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        if self.is_enabled() {
            trace!(
                "Filter '{:?}' is enabled for the current request under path '{}' with config [{:?}].
                                Delegating to subclass implementation for 'onPreHandle' check.",
                self.get_name(),
                path,
                path_config.as_ref().map(|s| s.to_string())
            );

            // The filter is enabled for this specific request, so delegate to subclass implementations
            // so they can decide if the request should continue through the chain or not:
            return match path_matching_filter_ext {
                Some(ext) => ext.on_pre_handle(request, response, path_config).await,
                None => true,
            };
        }

        trace!(
            "Filter '{:?}' is disabled for the current request under path '{}' with config [{:?}].
                                The next element in the FilterChain will be called immediately.",
            self.get_name(),
            path,
            path_config.map(|s| s.to_string())
        );

        // This filter is disabled for this specific request,
        // return 'true' immediately to indicate that the filter will not process the request
        // and let the request/response to continue through the filter chain:
        true
    }
}

#[async_trait]
impl<T> AdviceFilterExt for PathMatchingFilter<T>
where
    T: PatternMatcher,
{
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        if self.applied_paths.is_empty() {
            trace!("applied_paths property is null or empty.  This Filter will passthrough immediately.");

            return true;
        }

        for path in self.applied_paths.keys() {
            // If the path does match, then pass on to the subclass implementation for specific checks
            //(first match 'wins'):
            if self.paths_match(path, request) {
                trace!("Current requestURI matches pattern '{}'.  Determining filter chain execution...", path);
                let config = self.applied_paths.get(path);
                return self
                    .is_filter_chain_continued(request, response, path, config.cloned(), ext)
                    .await;
            }
        }

        // no path matched, allow the request to go through:
        true
    }
}

impl<T> Named for PathMatchingFilter<T> {
    fn name(&self) -> &str {
        "AdviceFilter"
    }
}

impl<T> Deref for PathMatchingFilter<T> {
    type Target = AdviceFilter;

    fn deref(&self) -> &Self::Target {
        &self.advice_filter
    }
}

impl<T> DerefMut for PathMatchingFilter<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.advice_filter
    }
}

impl<T> Default for PathMatchingFilter<T>
where
    T: PatternMatcher + Default,
{
    fn default() -> Self {
        Self {
            path_matcher: T::default(),
            applied_paths: IndexMap::new(),
            advice_filter: Default::default(),
        }
    }
}

#[async_trait]
pub trait PathMatchingFilterExt
where
    Self: Send + Sync,
{
    #[allow(unused_variables)]
    async fn on_pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        true
    }
}

#[async_trait]
impl<T> PathMatchingFilterExt for T
where
    T: AccessControlFilterExt,
{
    #[allow(unused_variables)]
    async fn on_pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        let _mapped_value = mapped_value.clone();
        self.is_access_allowed(request, response, mapped_value)
            .await
            || self
                .on_access_denied(request, response, _mapped_value)
                .await
    }
}
