use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{
    userdetails::{
        authentication_user_details_service::AuthenticationUserDetailsService,
        user_details::UserDetails, user_details_service::UserDetailsService,
    },
    Authentication,
};

use super::username_not_found_error::UsernameNotFoundError;

pub struct UserDetailsByNameServiceWrapper<T>
where
    T: Authentication + Send + Sync,
{
    user_details_service: Arc<dyn UserDetailsService>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> UserDetailsByNameServiceWrapper<T>
where
    T: Authentication + Send + Sync,
{
    pub fn new(user_details_service: Arc<dyn UserDetailsService>) -> Self {
        Self {
            user_details_service,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn after_properties_set(&self) {}

    pub fn set_user_details_service(&mut self, user_details_service: Arc<dyn UserDetailsService>) {
        self.user_details_service = user_details_service;
    }
}

#[async_trait]
impl<T> AuthenticationUserDetailsService<T> for UserDetailsByNameServiceWrapper<T>
where
    T: Authentication + Send + Sync,
{
    async fn load_user_details(
        &self,
        token: &T,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
        self.user_details_service
            .load_user_by_username(token.get_name())
            .await
    }
}
