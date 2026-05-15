use next_web_core::async_trait;

use crate::core::userdetails::user_details::UserDetails;

#[async_trait]
pub trait LdapUserDetails: UserDetails {
    async fn get_dn(&self) -> String;
}
