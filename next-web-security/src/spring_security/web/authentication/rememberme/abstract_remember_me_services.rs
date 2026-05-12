use crate::web::authentication::remember_me_services::RememberMeServices;

#[derive(Clone)]
pub struct AbstractRememberMeServices {}

impl AbstractRememberMeServices {
    pub fn get_parameter(&self) -> &str {
        "remember-me"
    }
}

impl RememberMeServices for AbstractRememberMeServices {}
