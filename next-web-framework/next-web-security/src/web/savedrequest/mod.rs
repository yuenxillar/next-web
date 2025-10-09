use std::collections::HashMap;

use next_web_core::util::locale::Locale;

pub mod http_session_request_cache;
pub mod request_cache;


pub trait SavedRequest: Send + Sync {
    fn get_redirect_url(&self) -> String;

    fn get_cookies(&self) -> Vec<Cookie>;

    fn get_method(&self) -> String;

    fn get_header_values(&self, name: &str)-> Vec<String>;

    fn get_header_names(&self) -> Vec<String>;

    fn  get_locales(&self) -> Vec<Locale>;

    fn get_parameter_values(&self, name: &str) -> Vec<String>;

    fn get_parameter_map(&self) -> HashMap<String, Vec<String>>;
}

pub struct Cookie {}