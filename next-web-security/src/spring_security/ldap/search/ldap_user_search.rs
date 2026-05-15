use next_web_core::async_trait;

use crate::{
    core::userdetails::username_not_found_error::UsernameNotFoundError,
    ldap::dir_context_operations::DirContextOperations,
};

#[async_trait]
pub trait LdapUserSearch: Send + Sync {
    async fn search_for_user(
        &self,
        username: &str,
    ) -> Result<DirContextOperations, UsernameNotFoundError>;
}
