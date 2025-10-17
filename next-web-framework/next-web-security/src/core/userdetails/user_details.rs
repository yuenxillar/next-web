use next_web_core::async_trait;

use crate::core::granted_authority::GrantedAuthority;

#[async_trait]
pub trait UserDetails 
where
    Self: Send + Sync,
{
    async fn get_authorities(&self) -> Vec<&dyn GrantedAuthority>;

    async fn get_password(&self)  -> String;

    async fn get_username(&self) -> String;

    async fn is_account_non_expired(&self)-> bool;

    async fn is_account_non_locked(&self)-> bool;

    async fn is_credentials_non_expired(&self)-> bool;

    async fn is_enabled(&self)-> bool;
}