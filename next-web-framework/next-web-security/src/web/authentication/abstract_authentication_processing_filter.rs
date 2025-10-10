use crate::config::web::util::matcher::request_matcher::RequestMatcher;


#[derive(Clone)]
pub struct AbstractAuthenticationProcessingFilter {}


impl AbstractAuthenticationProcessingFilter {
    
    pub fn set_requires_authentication_request_matcher(
        &mut self,
        request_matcher: impl RequestMatcher + 'static
    )  {

    }
}