use std::{collections::HashMap, sync::Arc};

use axum::{extract::Request, http::header};
use dashmap::DashMap;
use next_web_core::util::locale::Locale;

use crate::web::savedrequest::request_cache::RequestCache;

use super::{Cookie, SavedRequest};

pub struct HttpSessionRequestCache {
    requests: DashMap<String, Arc<DefaultSavedRequest>>,
}

impl HttpSessionRequestCache {
    pub fn new() -> Self {
        Self {
            requests: DashMap::new(),
        }
    }

    fn request_key(request: &Request) -> String {
        format!("{} {}", request.method().as_str(), request.uri())
    }
}

impl RequestCache for HttpSessionRequestCache {
    fn save_request(&self, request: &Request) {
        self.requests.insert(
            Self::request_key(request),
            Arc::new(DefaultSavedRequest::from_request(request)),
        );
    }

    fn get_request(&self, request: &Request) -> Option<Arc<dyn SavedRequest>> {
        self.requests
            .get(&Self::request_key(request))
            .map(|request| request.clone() as Arc<dyn SavedRequest>)
    }

    fn get_matching_request(&self, request: &Request) -> Option<Arc<dyn SavedRequest>> {
        self.get_request(request)
    }

    fn remove_request(&self, request: &Request) {
        self.requests.remove(&Self::request_key(request));
    }
}

struct DefaultSavedRequest {
    redirect_url: String,
    method: String,
    headers: HashMap<String, Vec<String>>,
    parameters: HashMap<String, Vec<String>>,
}

impl DefaultSavedRequest {
    fn from_request(request: &Request) -> Self {
        let headers = request
            .headers()
            .iter()
            .fold(HashMap::<String, Vec<String>>::new(), |mut map, (name, value)| {
                if let Ok(value) = value.to_str() {
                    map.entry(name.as_str().to_string())
                        .or_default()
                        .push(value.to_string());
                }
                map
            });

        let parameters = request
            .uri()
            .query()
            .map(parse_query)
            .unwrap_or_default();

        Self {
            redirect_url: request.uri().to_string(),
            method: request.method().as_str().to_string(),
            headers,
            parameters,
        }
    }
}

impl SavedRequest for DefaultSavedRequest {
    fn get_redirect_url(&self) -> String {
        self.redirect_url.clone()
    }

    fn get_cookies(&self) -> Vec<Cookie> {
        Vec::new()
    }

    fn get_method(&self) -> String {
        self.method.clone()
    }

    fn get_header_values(&self, name: &str) -> Vec<String> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .cloned()
            .or_else(|| self.headers.get(name).cloned())
            .unwrap_or_default()
    }

    fn get_header_names(&self) -> Vec<String> {
        self.headers.keys().cloned().collect()
    }

    fn get_locales(&self) -> Vec<Locale> {
        if self.headers.contains_key(header::ACCEPT_LANGUAGE.as_str()) {
            Vec::new()
        } else {
            Vec::new()
        }
    }

    fn get_parameter_values(&self, name: &str) -> Vec<String> {
        self.parameters.get(name).cloned().unwrap_or_default()
    }

    fn get_parameter_map(&self) -> HashMap<String, Vec<String>> {
        self.parameters.clone()
    }
}

fn parse_query(query: &str) -> HashMap<String, Vec<String>> {
    let mut parameters = HashMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let mut parts = pair.splitn(2, '=');
        let name = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        let name = urlencoding::decode(name)
            .map(|name| name.into_owned())
            .unwrap_or_else(|_| name.to_string());
        let value = urlencoding::decode(value)
            .map(|value| value.into_owned())
            .unwrap_or_else(|_| value.to_string());
        parameters
            .entry(name)
            .or_insert_with(Vec::new)
            .push(value);
    }
    parameters
}
