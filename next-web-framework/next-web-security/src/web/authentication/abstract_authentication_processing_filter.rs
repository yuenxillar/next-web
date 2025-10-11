use crate::{config::web::util::matcher::request_matcher::RequestMatcher, web::authentication::remember_me_services::RememberMeServices};


#[derive(Clone)]
pub struct AbstractAuthenticationProcessingFilter {}


impl AbstractAuthenticationProcessingFilter {
    pub fn get_remember_me_services<'a>(&'a self) -> &'a dyn RememberMeServices {
        
        todo!()
    }
}

impl AbstractAuthenticationProcessingFilter {
    
    pub fn set_requires_authentication_request_matcher(
        &mut self,
        request_matcher: impl RequestMatcher + 'static
    )  {

    }
}