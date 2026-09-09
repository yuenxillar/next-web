mod cookie_request_cache;
mod default_saved_request;
mod http_session_request_cache;
mod null_request_cache;
mod request_cache;
mod request_cache_aware_filter;
mod saved_request_aware_wrapper;

pub use cookie_request_cache::CookieRequestCache;
pub use default_saved_request::DefaultSavedRequest;
pub use http_session_request_cache::HttpSessionRequestCache;
pub use null_request_cache::NullRequestCache;
pub use request_cache::RequestCache;
pub use request_cache_aware_filter::RequestCacheAwareFilter;
pub(super) use saved_request_aware_wrapper::SavedRequestAwareWrapper;

pub trait SavedRequest
where
    Self: Send + Sync,
    Self: next_web_core::DynClone,
    Self: std::fmt::Debug,
{
    fn get_redirect_url(&self) -> String;

    fn get_cookies(&self) -> Vec<next_web_core::http::Cookie>;

    fn get_method(&self) -> &str;

    fn get_header_values(&self, name: &str) -> Vec<&str>;

    fn get_header_names(&self) -> Vec<&str>;

    fn get_locales(&self) -> Vec<next_web_core::util::locale::Locale>;

    fn get_parameter_values(&self, name: &str) -> Option<Vec<&str>>;

    fn get_parameter_map(&self) -> std::collections::HashMap<String, Vec<String>>;
}

next_web_core::clone_trait_object!(SavedRequest where Self: Send + Sync + std::fmt::Debug);
