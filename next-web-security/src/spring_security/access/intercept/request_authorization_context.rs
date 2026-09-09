use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use next_web_core::{http::HttpRequestShare, traits::http::http_request::HttpRequest};

/// An HttpServletRequest authorization context.
#[derive(Clone)]
pub struct RequestAuthorizationContext {
    /// Path variables are assigned at most once by the request matcher
    /// delegation step. `OnceLock` keeps reads and the first write lock-free.
    variables: Arc<OnceLock<BTreeMap<String, String>>>,
    request: HttpRequestShare,
}

impl RequestAuthorizationContext {
    /// Creates an instance.
    pub fn new(request: HttpRequestShare, variables: Option<BTreeMap<String, String>>) -> Self {
        let variable_cell = Arc::new(OnceLock::new());
        if let Some(variables) = variables {
            let _ = variable_cell.set(variables);
        }
        Self {
            variables: variable_cell,
            request,
        }
    }

    /// Returns the request.
    pub fn request(&self) -> &dyn HttpRequest {
        &self.request
    }

    /// Returns the extracted variable values where the key is the variable name and the value is the variable value.
    pub fn variables(&self) -> Option<&BTreeMap<String, String>> {
        self.variables.get()
    }

    pub fn set_variables(&self, variables: Option<BTreeMap<String, String>>) {
        if let Some(variables) = variables {
            let _ = self.variables.set(variables);
        }
    }
}

impl From<&mut dyn HttpRequest> for RequestAuthorizationContext {
    fn from(req: &mut dyn HttpRequest) -> Self {
        Self {
            variables: Arc::new(OnceLock::new()),
            request: req.shared().clone(),
        }
    }
}
