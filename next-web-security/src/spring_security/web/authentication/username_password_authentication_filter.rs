use next_web_core::traits::required::Required;

use crate::web::authentication::abstract_authentication_processing_filter::AbstractAuthenticationProcessingFilter;

#[derive(Clone)]
pub struct UsernamePasswordAuthenticationFilter {
    username_parameter: Box<str>,
    password_parameter: Box<str>,
    post_only: bool,
    abstract_authentication_processing_filter: AbstractAuthenticationProcessingFilter,
}

impl Default for UsernamePasswordAuthenticationFilter {
    fn default() -> Self {
        Self {
            username_parameter: "username".into(),
            password_parameter: "password".into(),
            post_only: true,
            abstract_authentication_processing_filter:
                AbstractAuthenticationProcessingFilter::default(),
        }
    }
}

impl UsernamePasswordAuthenticationFilter {
    pub fn set_username_parameter(&mut self, username_parameter: &str) {
        assert!(
            !username_parameter.trim().is_empty(),
            "username_parameter cannot be empty"
        );
        self.username_parameter = username_parameter.into();
    }

    pub fn set_password_parameter(&mut self, password_parameter: &str) {
        assert!(
            !password_parameter.trim().is_empty(),
            "password_parameter cannot be empty"
        );
        self.password_parameter = password_parameter.into();
    }

    pub fn get_username_parameter(&self) -> &str {
        &self.username_parameter
    }

    pub fn get_password_parameter(&self) -> &str {
        &self.password_parameter
    }
}
impl Required<AbstractAuthenticationProcessingFilter> for UsernamePasswordAuthenticationFilter {
    fn get_object(&self) -> &AbstractAuthenticationProcessingFilter {
        &self.abstract_authentication_processing_filter
    }

    fn get_mut_object(&mut self) -> &mut AbstractAuthenticationProcessingFilter {
        &mut self.abstract_authentication_processing_filter
    }
}
