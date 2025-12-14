
#[derive(Clone)]
pub struct AbstractRememberMeServices {}

impl AbstractRememberMeServices {
    
    pub fn get_parameter(&self) -> &str {
        "remember-me"
    }
}