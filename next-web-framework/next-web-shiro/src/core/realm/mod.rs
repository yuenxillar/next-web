use next_web_core::async_trait;

use crate::core::authc::{
    authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
};

pub mod authenticating_realm;
pub mod authorizing_realm;
pub mod caching_realm;

#[async_trait]
pub trait Realm {
    fn get_name(&self) -> &str;

    fn supports(&self, authentication_token: &dyn AuthenticationToken) -> bool;

    async fn get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>>;
}
