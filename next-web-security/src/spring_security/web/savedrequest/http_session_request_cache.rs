use std::sync::Arc;

use next_web_core::{
    anys::any_value::AnyValue,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::StringUtils,
};
use tracing::{debug, trace, Level};

use crate::web::{
    savedrequest::request_cache::RequestCache,
    savedrequest::{DefaultSavedRequest, SavedRequest, SavedRequestAwareWrapper},
    util::matcher::{AnyRequestMatcher, RequestMatcher},
    util::UrlUtils,
};

const SAVED_REQUEST: &str = "NEXT_SECURITY_SAVED_REQUEST";

///
/// RequestCache which stores the SavedRequest in the HttpSession.
/// The DefaultSavedRequest class is used as the implementation.
pub struct HttpSessionRequestCache {
    create_session_allowed: bool,
    request_matcher: Arc<dyn RequestMatcher>,
    session_attr_name: Box<str>,
    matching_request_parameter_name: Box<str>,
}

impl HttpSessionRequestCache {
    /// Allows selective use of saved requests for a subset of requests. By default any request will be cached by the saveRequest method.
    /// If set, only matching requests will be cached.
    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }

    /// If true, indicates that it is permitted to store the target URL and exception information in a new
    /// HttpSession (the default). In situations where you do not wish to unnecessarily
    /// create HttpSessions - because the user agent will know the failed URL, such as with BASIC or Digest
    /// authentication - you may wish to set this property to false.
    pub fn set_create_session_allowed(&mut self, create_session_allowed: bool) {
        self.create_session_allowed = create_session_allowed;
    }

    /// If the sessionAttrName property is set, the request is stored in the session using this attribute
    /// name. Default is "NEXT_SECURITY_SAVED_REQUEST".
    pub fn set_session_attr_name(&mut self, session_attr_name: impl Into<Box<str>>) {
        self.session_attr_name = session_attr_name.into();
    }

    /// Specify the name of a query parameter that is added to the URL that specifies the request cache should be checked in get_matching_request(HttpRequest, HttpResponse)
    pub fn set_matching_request_parameter_name(
        &mut self,
        matching_request_parameter_name: impl Into<Box<str>>,
    ) {
        self.matching_request_parameter_name = matching_request_parameter_name.into();
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
    fn save_request(&self, request: &mut dyn HttpRequest, _response: &mut dyn HttpResponse) {
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
            let saved_request = DefaultSavedRequest::from_request(
                request,
                Some(&self.matching_request_parameter_name),
            );
            if let Some(session) = request.session_mut(true) {
                session.set_attribute(
                    &self.session_attr_name,
                    AnyValue::Object(Box::new(Arc::new(saved_request) as Arc<dyn SavedRequest>)),
                );
            }
            if tracing::enabled!(Level::DEBUG) {
                debug!("Saved request to session");
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
            .and_then(|req| req.as_object::<Arc<dyn SavedRequest>>())
    }

    fn get_matching_request<'a>(
        &self,
        request: &'a mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Box<dyn HttpRequest + 'a>> {
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

        Some(Box::new(SavedRequestAwareWrapper::new(saved, request)))
    }

    fn remove_request(&self, request: &dyn HttpRequest, _response: &mut dyn HttpResponse) {
        if let Some(session) = request.session() {
            trace!("Removing DefaultSavedRequest from session if present");
            session.remove_attribute(&self.session_attr_name);
        }
    }
}
