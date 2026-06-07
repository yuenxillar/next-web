use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{
    Authentication,
    userdetails::{
        authentication_user_details_service::AuthenticationUserDetailsService,
        user_details::UserDetails, user_details_service::UserDetailsService,
    },
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use next_web_core::async_trait;

    use crate::core::{
        authority_utils::AuthorityUtils,
        userdetails::{
            authentication_user_details_service::AuthenticationUserDetailsService,
            user::User,
            user_details_service::UserDetailsService,
            user_details_by_name_service_wrapper::UserDetailsByNameServiceWrapper,
            username_not_found_error::UsernameNotFoundError,
        },
        username_password_authentication_token::UsernamePasswordAuthenticationToken,
    };

    struct StubUserDetailsService {
        users: HashMap<String, Arc<User>>,
    }

    #[async_trait]
    impl UserDetailsService for StubUserDetailsService {
        async fn load_user_by_username(
            &self,
            username: String,
        ) -> Result<Arc<dyn crate::core::userdetails::user_details::UserDetails>, UsernameNotFoundError>
        {
            self.users
                .get(&username)
                .cloned()
                .map(|user| user as Arc<dyn crate::core::userdetails::user_details::UserDetails>)
                .ok_or(UsernameNotFoundError(username))
        }
    }

    #[tokio::test]
    async fn wrapper_loads_by_authentication_name() {
        let wrapper = UserDetailsByNameServiceWrapper::<UsernamePasswordAuthenticationToken>::new(
            Arc::new(StubUserDetailsService {
                users: HashMap::from([(
                    String::from("alice"),
                    Arc::new(User::new(
                        "alice",
                        Some(String::from("secret")),
                        AuthorityUtils::create_authority_list(["ROLE_USER"]),
                    )),
                )]),
            }),
        );

        let token = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );

        let user = wrapper.load_user_details(&token).await.unwrap();
        assert_eq!(user.get_username().await, "alice");
    }
}
