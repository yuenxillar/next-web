use std::collections::BTreeMap;

use next_web_core::{http::HttpRequestShare, traits::http::http_request::HttpRequest};

/// An HttpServletRequest authorization context.
#[derive(Clone)]
pub struct RequestAuthorizationContext {
    variables: Option<BTreeMap<String, String>>,
    request: HttpRequestShare,
}

impl RequestAuthorizationContext {
    /// Creates an instance.
    pub fn new(request: HttpRequestShare, variables: Option<BTreeMap<String, String>>) -> Self {
        Self { variables, request }
    }

    /// Returns the request.
    pub fn request(&self) -> &HttpRequestShare {
        &self.request
    }

    /// Returns the extracted variable values where the key is the variable name and the value is the variable value.
    pub fn variables(&self) -> Option<&BTreeMap<String, String>> {
        self.variables.as_ref()
    }

    pub fn set_variables(&mut self, variables: Option<BTreeMap<String, String>>) {
        self.variables = variables;
    }
}

impl From<&mut dyn HttpRequest> for RequestAuthorizationContext {
    fn from(req: &mut dyn HttpRequest) -> Self {
        Self {
            variables: None,
            request: req.shared().clone(),
        }
    }
}
