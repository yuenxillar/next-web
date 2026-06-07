use std::{collections::HashMap, fmt::Debug};

use next_web_core::{clone_trait_object, http::cookie::Cookie, util::locale::Locale, DynClone};

pub trait SavedRequest
where
    Self: Send + Sync,
    Self: DynClone,
    Self: Debug,
{
    fn get_redirect_url(&self) -> String;

    fn get_cookies(&self) -> Vec<Cookie>;

    fn get_method(&self) -> String;

    fn get_header_values(&self, name: &str) -> Vec<String>;

    fn get_header_names(&self) -> Vec<String>;

    fn get_locales(&self) -> Vec<Locale>;

    fn get_parameter_values(&self, name: &str) -> Vec<String>;

    fn get_parameter_map(&self) -> HashMap<String, Vec<String>>;
}
clone_trait_object!(SavedRequest where Self: Send + Sync + Debug);

mod cookie_request_cache;
mod default_saved_request;
mod http_session_request_cache;
mod request_cache;
mod request_cache_aware_filter;
mod saved_request_aware_wrapper;

pub use cookie_request_cache::CookieRequestCache;
pub use default_saved_request::DefaultSavedRequest;
pub use http_session_request_cache::HttpSessionRequestCache;
pub use request_cache::RequestCache;
pub use request_cache_aware_filter::RequestCacheAwareFilter;
pub(super) use saved_request_aware_wrapper::SavedRequestAwareWrapper;
