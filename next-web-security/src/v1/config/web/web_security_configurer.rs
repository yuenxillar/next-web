use next_web_core::{clone_trait_object, DynClone};

use crate::config::web::builders::HttpSecurity;

pub trait WebSecurityConfigurer
where
    Self: DynClone,
    Self: Send + Sync,
{
    fn configure(&mut self, http: &mut HttpSecurity);
}

clone_trait_object!(WebSecurityConfigurer where Self: Send + Sync);
