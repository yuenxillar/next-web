use next_web_core::DynClone;

use crate::config::web::http_security::HttpSecurity;

pub trait WebSecurityConfigure: DynClone + Send + Sync {
    
    fn configure(self) -> HttpSecurity;
}

next_web_core::clone_trait_object!(WebSecurityConfigure);