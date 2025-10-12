use next_web_core::traits::required::Required;

use crate::web::authentication::abstract_authentication_processing_filter::AbstractAuthenticationProcessingFilter;

#[derive(Clone)]
pub struct UsernamePasswordAuthenticationFilter {
    username_parameter: Box<str>,
    password_parameter: Box<str>,
    post_only: bool,
}

impl Default for UsernamePasswordAuthenticationFilter {
    fn default() -> Self {
        Self {
            username_parameter: "username".into(),
            password_parameter: "password".into(),
            post_only: true,
        }
    }
}

impl UsernamePasswordAuthenticationFilter {
    pub fn set_username_parameter(&mut self, username_parameter: &str) {}

    pub fn set_password_parameter(&mut self, password_parameter: &str) {}

    pub fn get_username_parameter(&self) -> &str {
        &self.username_parameter
    }

    pub fn get_password_parameter(&self) -> &str {
        &self.password_parameter
    }
}
impl Required<AbstractAuthenticationProcessingFilter> for UsernamePasswordAuthenticationFilter {
    fn get_object(&self) -> &AbstractAuthenticationProcessingFilter {
        todo!()
    }

    fn get_object_mut(&mut self) -> &mut AbstractAuthenticationProcessingFilter {
        todo!()
    }
}
