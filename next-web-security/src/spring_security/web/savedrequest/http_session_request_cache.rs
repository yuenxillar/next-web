use std::{collections::HashMap, sync::Arc};

use next_web_core::http::Cookie;
use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};
use next_web_core::util::locale::Locale;
use next_web_core::util::StringUtils;
use tracing::{debug, trace, Level};

use crate::web::savedrequest::request_cache::RequestCache;
use crate::web::util::matcher::{AnyRequestMatcher, RequestMatcher};
use crate::web::util::UrlUtils;

use super::SavedRequest;

const SAVED_REQUEST: &str = "NEXT_SECURITY_SAVED_REQUEST";

pub struct HttpSessionRequestCache {
    create_session_allowed: bool,
    request_matcher: Arc<dyn RequestMatcher>,
    session_attr_name: Box<str>,
    matching_request_parameter_name: Box<str>,
}

impl HttpSessionRequestCache {
    fn request_key(request: &dyn HttpRequest) -> String {
        let uri = request.uri();
        format!("{} {}", request.method().to_string(), uri)
    }

    pub fn set_session_attr_name(&mut self, session_attr_name: impl Into<Box<str>>) {
        self.session_attr_name = session_attr_name.into();
    }

    pub fn set_matching_request_parameter_name(
        &mut self,
        matching_request_parameter_name: impl Into<Box<str>>,
    ) {
        self.matching_request_parameter_name = matching_request_parameter_name.into();
    }

    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }

    pub fn set_create_session_allowed(&mut self, create_session_allowed: bool) {
        self.create_session_allowed = create_session_allowed;
    }

    fn matches_saved_request(
        &self,
        request: &dyn HttpRequest,
        saved_request: &dyn SavedRequest,
    ) -> bool {
        let current_url = UrlUtils::build_full_request_url(request);
        saved_request.get_redirect_url() == (current_url)
    }
}

impl Default for HttpSessionRequestCache {
    fn default() -> Self {
        Self {
            create_session_allowed: true,
            request_matcher: AnyRequestMatcher::instance(),
            session_attr_name: SAVED_REQUEST.into(),
            matching_request_parameter_name: "continue".into(),
        }
    }
}

impl RequestCache for HttpSessionRequestCache {
    fn save_request(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        if !self.request_matcher.matches(request) {
            if tracing::enabled!(Level::TRACE) {
                trace!(
                    "Did not save request since it did not match [{:?}]",
                    self.request_matcher
                );
            }
            return;
        }

        if self.create_session_allowed || request.session().is_some() {
            let saved_request = "";
            if tracing::enabled!(Level::TRACE) {
                trace!("Saved request {} to session", saved_request);
            }
        } else {
            trace!(
                "Did not save request since there's no session and create_session_allowed is false"
            );
        }
    }

    fn get_request(
        &self,
        request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn SavedRequest>> {
        let session = match request.session() {
            Some(s) => s,
            None => return None,
        };
        session
            .attribute(&self.session_attr_name)
            .map(|req| req.as_object::<Arc<dyn SavedRequest>>())
            .unwrap_or_default()
    }

    fn get_matching_request(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Box<dyn HttpRequest>> {
        if let Some(query) = request.query() {
            if !StringUtils::has_text(query)
                || request
                    .parameter(&self.matching_request_parameter_name)
                    .is_none()
            {
                trace!("matchingRequestParameterName is required for getMatchingRequest to lookup a value, but not provided");
                return None;
            }
        }

        let saved = match self.get_request(request, response) {
            Some(saved) => saved,
            None => {
                trace!("No saved request found for get_request");
                return None;
            }
        };

        if !self.matches_saved_request(request, saved.as_ref()) {
            if tracing::enabled!(Level::TRACE) {
                trace!(
                    "Did not match request {} to the saved one {:?}",
                    UrlUtils::build_request_url(request),
                    saved
                );
            }
            return None;
        }

        self.remove_request(request, response);

        if tracing::enabled!(Level::DEBUG) {
            debug!("Loaded matching saved request {}", saved.get_redirect_url());
        }

        todo!()
    }

    fn remove_request(&self, request: &dyn HttpRequest, _response: &mut dyn HttpResponse) {
        if let Some(session) = request.session() {
            trace!("Removing DefaultSavedRequest from session if present");
            session.remove_attribute(&self.session_attr_name);
        }
    }
}

#[derive(Debug, Clone)]
struct DefaultSavedRequest {
    redirect_url: String,
    method: String,
    headers: HashMap<String, Vec<String>>,
    parameters: HashMap<String, Vec<String>>,
}

impl DefaultSavedRequest {
    fn from_request(request: &dyn HttpRequest) -> Self {
        let parameters = request.query().map(parse_query).unwrap_or_default();

        let redirect_url = request.uri().to_string();
        let method = request.method().to_string();

        Self {
            redirect_url,
            method,
            headers: HashMap::new(),
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
        Vec::new()
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
        parameters.entry(name).or_insert_with(Vec::new).push(value);
    }
    parameters
}
