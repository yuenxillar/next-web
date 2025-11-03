use crate::web::authentication_entry_point::AuthenticationEntryPoint;


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
        Self {
            login_form_url,
        }
    }
}

impl AuthenticationEntryPoint for LoginUrlAuthenticationEntryPoint {
    fn commence(
        &self,
        request: &mut axum::extract::Request,
        response: &mut axum::response::Response,
        auth_error: Option<crate::core::authentication_error::AuthenticationError>,
    ) -> Result<(), next_web_core::error::BoxError> {
        todo!()
    }
}
