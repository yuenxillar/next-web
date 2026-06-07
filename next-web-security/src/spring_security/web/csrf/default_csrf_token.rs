use crate::web::csrf::CsrfToken;

/// CSRF Token
#[derive(Debug, Clone)]
pub struct DefaultCsrfToken {
    header_name: String,
    parameter_name: String,
    token: String,
}

impl DefaultCsrfToken {
    pub fn new(
        header_name: impl Into<String>,
        parameter_name: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        let header_name = header_name.into();
        let parameter_name = parameter_name.into();
        let token = token.into();

        assert!(
            !header_name.is_empty(),
            "headerName cannot be null or empty"
        );
        assert!(
            !parameter_name.is_empty(),
            "parameterName cannot be null or empty"
        );
        assert!(!token.is_empty(), "token cannot be null or empty");

        Self {
            header_name,
            parameter_name,
            token,
        }
    }
}

impl CsrfToken for DefaultCsrfToken {
    fn get_header_name(&self) -> &str {
        self.header_name.as_str()
    }

    fn get_parameter_name(&self) -> &str {
        self.parameter_name.as_str()
    }

    fn get_token(&self) -> &str {
        self.token.as_str()
    }
}
