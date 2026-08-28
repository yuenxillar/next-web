use std::{marker::PhantomData, sync::Arc};

use next_web_core::async_trait;

use crate::core::{
    userdetails::{
        authentication_user_details_service::AuthenticationUserDetailsService,
        user_details::UserDetails, user_details_service::UserDetailsService,
    },
    Authentication, AuthenticationError,
};

/// This implementation for AuthenticationUserDetailsService wraps a regular Spring Security
/// UserDetailsService implementation, to retrieve a UserDetails object based on the user name contained in
/// an Authentication object.
pub struct UserDetailsByNameServiceWrapper<T>
where
    T: Authentication,
{
    user_details_service: Arc<dyn UserDetailsService>,
    _marker: PhantomData<T>,
}

impl<T> UserDetailsByNameServiceWrapper<T>
where
    T: Authentication,
{
    /// Constructs a new wrapper using the supplied UserDetailsService as the service to delegate to.
    pub fn new(user_details_service: Arc<dyn UserDetailsService>) -> Self {
        Self {
            user_details_service,
            _marker: PhantomData,
        }
    }

    /// Set the wrapped UserDetailsService implementation
    pub fn set_user_details_service(&mut self, user_details_service: Arc<dyn UserDetailsService>) {
        self.user_details_service = user_details_service;
    }
}

#[async_trait]
impl<T> AuthenticationUserDetailsService<T> for UserDetailsByNameServiceWrapper<T>
where
    T: Authentication,
{
    /// Get the UserDetails object from the wrapped UserDetailsService implementation
    async fn load_user_details(
        &self,
        authentication: &T,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        self.user_details_service
            .load_user_by_username(authentication.name())
            .await
    }
}
