use next_web_core::traits::Required::Required;

use crate::web::authentication::abstract_authentication_processing_filter::AbstractAuthenticationProcessingFilter;


#[derive(Clone)]
pub struct UsernamePasswordAuthenticationFilter {}


impl Default for UsernamePasswordAuthenticationFilter {
    fn default() -> Self {
        Self {  }
    }
}


impl UsernamePasswordAuthenticationFilter {
    pub fn set_username_parameter(&mut self)
}
impl Required<AbstractAuthenticationProcessingFilter> for UsernamePasswordAuthenticationFilter{
    fn get_object(&self) -> & AbstractAuthenticationProcessingFilter {
        todo!()
    }

    fn get_object_mut(&mut self) -> &mut AbstractAuthenticationProcessingFilter {
        todo!()
    }
}