use next_web_core::DynClone;

use crate::config::web::builders::HttpSecurity;

pub trait WebSecurityConfigure
where
    Self: DynClone,
    Self: Send + Sync,
{
    fn configure(&mut self) -> HttpSecurity;
}

next_web_core::clone_trait_object!(WebSecurityConfigure);
