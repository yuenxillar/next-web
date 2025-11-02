use crate::core::mgt::security_manager::SecurityManager;


pub trait WebSecurityManager
where 
Self: SecurityManager
{
    fn is_http_session_mode(&self) -> bool;
}