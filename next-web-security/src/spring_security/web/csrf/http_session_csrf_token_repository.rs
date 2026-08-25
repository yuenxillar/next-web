use next_web_core::anys::any_value::AnyValue;
use next_web_core::error::BoxError;
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::{async_trait, traits::http::http_response::HttpResponse};
use std::sync::Arc;
use uuid::Uuid;

use crate::web::csrf::{CsrfToken, CsrfTokenRepository, DefaultCsrfToken};

/// A CsrfTokenRepository that stores the CsrfToken in the HttpSession.
#[derive(Clone)]
pub struct HttpSessionCsrfTokenRepository {
    parameter_name: String,
    header_name: String,
    session_attribute_name: String,
}

impl HttpSessionCsrfTokenRepository {
    const DEFAULT_CSRF_PARAMETER_NAME: &'static str = "_csrf";
    const DEFAULT_CSRF_HEADER_NAME: &'static str = "X-CSRF-TOKEN";
    const DEFAULT_CSRF_TOKEN_ATTR_NAME: &str =
        "next-web-security.web.csrf.HttpSessionCsrfTokenRepository.CSRF_TOKEN";

    /// Sets the HttpRequest parameter name that the CsrfToken is expected to appear on
    pub fn set_parameter_name(&mut self, parameter_name: impl Into<String>) {
        let parameter_name = parameter_name.into();
        assert!(!parameter_name.is_empty(), "parameterName cannot be  empty");
        self.parameter_name = parameter_name;
    }

    /// Sets the header name that the CsrfToken is expected to appear on and the header that the response will contain the CsrfToken.
    pub fn set_header_name(&mut self, header_name: impl Into<String>) {
        let header_name = header_name.into();
        assert!(!header_name.is_empty(), "headerName cannot be empty");
        self.header_name = header_name;
    }

    /// Sets the HttpSession attribute name that the CsrfToken is stored in
    pub fn set_session_attribute_name(&mut self, session_attribute_name: impl Into<String>) {
        let session_attribute_name = session_attribute_name.into();
        assert!(
            !session_attribute_name.is_empty(),
            "sessionAttributeName cannot be empty"
        );
        self.session_attribute_name = session_attribute_name;
    }

    fn create_new_token() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn parameter_name(&self) -> &str {
        &self.parameter_name
    }

    pub fn header_name(&self) -> &str {
        &self.header_name
    }

    pub fn session_attribute_name(&self) -> &str {
        &self.session_attribute_name
    }
}

impl Default for HttpSessionCsrfTokenRepository {
    fn default() -> Self {
        Self {
            parameter_name: Self::DEFAULT_CSRF_PARAMETER_NAME.to_string(),
            header_name: Self::DEFAULT_CSRF_HEADER_NAME.to_string(),
            session_attribute_name: Self::DEFAULT_CSRF_TOKEN_ATTR_NAME.to_string(),
        }
    }
}

#[async_trait]
impl CsrfTokenRepository for HttpSessionCsrfTokenRepository {
    async fn generate_token(&self, _: &mut dyn HttpRequest) -> Arc<dyn CsrfToken> {
        Arc::new(DefaultCsrfToken::new(
            self.header_name.as_str(),
            self.parameter_name.as_str(),
            Self::create_new_token(),
        ))
    }

    async fn save_token(
        &self,
        token: Option<&Arc<dyn CsrfToken>>,
        request: &mut dyn HttpRequest,
        _: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        match token {
            None => {
                if let Some(session) = request.session() {
                    session.remove_attribute(&self.session_attribute_name);
                }
            }
            Some(token) => {
                let Some(session) = request.session_mut(true) else {
                    return Ok(());
                };
                session.set_attribute(
                    &self.session_attribute_name,
                    AnyValue::Object(Box::new(Arc::clone(token))),
                );
            }
        }

        Ok(())
    }

    async fn load_token(&self, request: &mut dyn HttpRequest) -> Option<Arc<dyn CsrfToken>> {
        let session = request.session()?;
        session
            .attribute(&self.session_attribute_name)
            .and_then(|value| value.as_object::<Arc<dyn CsrfToken>>())
    }
}
