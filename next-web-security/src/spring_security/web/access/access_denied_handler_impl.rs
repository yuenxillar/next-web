use next_web_core::{
    anys::any_value::AnyValue,
    error::BoxError,
    http::StatusCode,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::StringUtils,
};
use tracing::{debug, enabled, trace, Level};

use crate::{
    access::AccessDeniedError,
    web::{access::AccessDeniedHandler, WebAttributes},
};

/// Base implementation of AccessDeniedHandler.
///
/// This implementation sends a 403 (SC_FORBIDDEN) HTTP error code. In addition,
/// if an errorPage is defined, the implementation will perform a request dispatcher "forward"
/// to the specified error page view. Being a "forward", the SecurityContextHolder will
/// remain populated. This is of benefit if the view (or a tag library or macro) wishes to access the SecurityContextHolder.
/// The request scope will also be populated with the exception itself, available from the key WebAttributes.ACCESS_DENIED_403.
#[derive(Default)]
pub struct AccessDeniedHandlerImpl {
    error_page: Option<String>,
}

impl AccessDeniedHandlerImpl {
    /// The error page to use. Must begin with a "/" and is interpreted relative to the current context root.
    pub fn set_error_page(&mut self, error_page: impl Into<String>) {
        let error_page = error_page.into();
        assert!(
            error_page.starts_with("/"),
            "error_page must begin with '/'"
        );
        self.error_page = Some(error_page);
    }
}
impl AccessDeniedHandler for AccessDeniedHandlerImpl {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        access_denied_error: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        if response.is_committed() {
            trace!("Did not write to response since already committed");
            return Ok(());
        }

        if !self
            .error_page
            .as_ref()
            .map(|s| StringUtils::has_text(s.as_str()))
            .unwrap_or_default()
        {
            debug!("Responding with 403 status code");

            response.set_status_code(StatusCode::FORBIDDEN);
            response.set_body(b"Forbidden".to_vec());
            return Ok(());
        }

        // Put exception into request scope (perhaps of use to a view)
        request.set_attribute(
            WebAttributes::ACCESS_DENIED_403,
            AnyValue::Object(Box::new(access_denied_error.clone())),
        );
        // Set the 403 status code.
        response.set_status_code(StatusCode::FORBIDDEN);

        // forward to error page.
        if enabled!(Level::DEBUG) {
            debug!(
                "Forwarding to {} with status code 403",
                self.error_page.as_deref().unwrap_or_default()
            );
        }

        if let Some(dispatcher) = self
            .error_page
            .as_deref()
            .and_then(|page| request.request_dispatcher(page))
        {
            dispatcher.forward(request, response)?;
        }

        Ok(())
    }
}
