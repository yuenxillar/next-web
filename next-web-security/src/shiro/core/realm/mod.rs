pub mod simple_account_realm;
use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::authc::{
    authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
};

pub mod authenticating_realm;
pub mod authorizing_realm;
pub mod caching_realm;

#[async_trait]
pub trait Realm
where 
Self: Send + Sync,
{
    fn get_name(&self) -> &str;

    fn supports(&self, authentication_token: &dyn AuthenticationToken) -> bool;

    async fn get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>>;
}

#[async_trait]
impl Realm for Arc<dyn Realm> {
    fn get_name(&self) ->  &str {
        self.as_ref().get_name()
    }

    fn supports(&self,authentication_token: &dyn AuthenticationToken) -> bool {
        self.as_ref().supports(authentication_token)
    }

     async fn get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>> {
        self.as_ref().get_authentication_info(token).await
    }
}