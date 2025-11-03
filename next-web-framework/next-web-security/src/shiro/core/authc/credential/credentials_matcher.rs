use crate::core::authc::{authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken};



pub trait CredentialsMatcher: Send + Sync {
    
    fn do_credentials_match(&self, token: &dyn AuthenticationToken, info: &dyn AuthenticationInfo) -> bool;
}