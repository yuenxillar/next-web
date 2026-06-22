use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{
    core::authentication_error::AuthenticationError,
    web::{
        authentication_entry_point::AuthenticationEntryPoint,
        redirect_strategy::{DefaultRedirectStrategy, RedirectStrategy},
    },
};

#[derive(Clone)]
pub struct LoginUrlAuthenticationEntryPoint {
    login_form_url: Box<str>,
}

impl LoginUrlAuthenticationEntryPoint {
    pub fn new(login_form_url: &str) -> Self {
        assert!(
            !login_form_url.trim().is_empty(),
            "login_form_url cannot be null"
        );
        let login_form_url = login_form_url.into();
        Self { login_form_url }
    }
}

impl AuthenticationEntryPoint for LoginUrlAuthenticationEntryPoint {
    fn commence(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _auth_error: Option<AuthenticationError>,
    ) -> Result<(), next_web_core::error::BoxError> {
        DefaultRedirectStrategy::default().send_redirect(request, response, &self.login_form_url);
        Ok(())
    }
}
