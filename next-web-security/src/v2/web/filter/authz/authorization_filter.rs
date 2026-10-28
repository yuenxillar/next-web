use next_web_core::{
    async_trait,
    http::{HttpRequest, HttpResponse, StatusCode},
};

use crate::{
    core::util::{object::Object, web::WebUtils},
    web::filter::access_control_filter::{AccessControlFilter, AccessControlFilterExt},
};

#[derive(Clone)]
pub struct AuthorizationFilter {
    unauthorized_url: Option<String>,
    pub(crate) access_control_filter: AccessControlFilter,
}

impl AuthorizationFilter {
    pub fn get_unauthorized_url(&self) -> Option<&str> {
        self.unauthorized_url.as_deref()
    }

    pub fn set_unauthorized_url(&mut self, unauthorized_url: impl ToString) {
        self.unauthorized_url = Some(unauthorized_url.to_string());
    }

    /// Applies the unauthorized URL of the application when this filter has none
    /// of its own.
    ///
    /// Returns `true` when the URL was taken, and `false` when the filter keeps
    /// the one the application configured on it.
    pub(crate) fn apply_unauthorized_url_if_absent(&mut self, unauthorized_url: &str) -> bool {
        if self.unauthorized_url.is_some() {
            return false;
        }

        self.set_unauthorized_url(unauthorized_url);

        true
    }
}

#[async_trait]
impl AccessControlFilterExt for AuthorizationFilter {
    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        // The request is over: the filter chain stops here, and the client is
        // either sent to the page the application configured for a refused
        // request or told that the request is not authorized — which is what
        // Apache Shiro's `AuthorizationFilter` answers.
        match self.get_unauthorized_url() {
            Some(url) if !url.is_empty() => {
                WebUtils::issue_redirect(request, response, url);
            }
            _ => response.set_status_code(StatusCode::UNAUTHORIZED),
        }

        false
    }
}

impl Default for AuthorizationFilter {
    fn default() -> Self {
        Self {
            unauthorized_url: Default::default(),
            access_control_filter: Default::default(),
        }
    }
}
