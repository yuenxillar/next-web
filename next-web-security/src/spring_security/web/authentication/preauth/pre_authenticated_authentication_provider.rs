use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::authentication_provider::AuthenticationProvider,
    core::{
        authentication::Authentication,
        authentication_error::AuthenticationError,
        authority_utils::AuthorityUtils,
        userdetails::{
            authentication_user_details_service::AuthenticationUserDetailsService,
            user_details_checker::UserDetailsChecker,
        },
    },
    web::authentication::preauth::{
        pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken,
        pre_authenticated_credentials_not_found_exception::pre_authenticated_credentials_not_found,
    },
};

pub struct PreAuthenticatedAuthenticationProvider {
    pre_authenticated_user_details_service:
        Arc<dyn AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>>,
    user_details_checker: Arc<dyn UserDetailsChecker>,
    granted_authorities: Vec<String>,
    throw_exception_when_token_rejected: bool,
    order: i32,
}

impl PreAuthenticatedAuthenticationProvider {
    pub fn new(
        pre_authenticated_user_details_service: Arc<
            dyn AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>,
        >,
        user_details_checker: Arc<dyn UserDetailsChecker>,
    ) -> Self {
        Self {
            pre_authenticated_user_details_service,
            user_details_checker,
            granted_authorities: Vec::new(),
            throw_exception_when_token_rejected: false,
            order: -1,
        }
    }

    pub fn set_throw_exception_when_token_rejected(&mut self, value: bool) {
        self.throw_exception_when_token_rejected = value;
    }

    pub fn set_granted_authorities(&mut self, authorities: Vec<String>) {
        self.granted_authorities = authorities;
    }

    pub fn order(&self) -> i32 {
        self.order
    }

    pub fn set_order(&mut self, order: i32) {
        self.order = order;
    }
}

#[async_trait]
impl AuthenticationProvider for PreAuthenticatedAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let Some(authentication) = authentication
            .as_any()
            .downcast_ref::<PreAuthenticatedAuthenticationToken>()
        else {
            return Err(AuthenticationError::new(
                "Only PreAuthenticatedAuthenticationToken is supported",
            ));
        };

        if authentication.get_principal().is_none() {
            if self.throw_exception_when_token_rejected {
                return Err(pre_authenticated_credentials_not_found(
                    "No pre-authenticated principal found in request.",
                ));
            }
            return Err(AuthenticationError::new(
                "No pre-authenticated principal found in request.",
            ));
        }
        if authentication.get_credentials().is_none() {
            if self.throw_exception_when_token_rejected {
                return Err(pre_authenticated_credentials_not_found(
                    "No pre-authenticated credentials found in request.",
                ));
            }
            return Err(AuthenticationError::new(
                "No pre-authenticated credentials found in request.",
            ));
        }

        let user_details = self
            .pre_authenticated_user_details_service
            .load_user_details(authentication)
            .await
            .map_err(|error| AuthenticationError::new(error.to_string()))?;
        self.user_details_checker.check(user_details.as_ref()).await?;

        let mut authorities = Vec::new();
        for authority in user_details.get_authorities().await {
            if let Some(name) = authority.get_authority().await {
                authorities.push(name);
            }
        }
        authorities.extend(self.granted_authorities.iter().cloned());

        let mut result = PreAuthenticatedAuthenticationToken::authenticated(
            user_details.get_username().await,
            authentication.get_credentials(),
            AuthorityUtils::create_authority_list(authorities),
        );
        result.set_details_value(authentication.get_details_value());
        Ok(Arc::new(result))
    }

    fn supports(&self, authentication: &str) -> bool {
        authentication == std::any::type_name::<PreAuthenticatedAuthenticationToken>()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::async_trait;

    use crate::{
        authentication::account_status_user_details_checker::AccountStatusUserDetailsChecker,
        authentication::authentication_provider::AuthenticationProvider,
        core::{
            authentication::Authentication,
            authority_utils::AuthorityUtils,
            userdetails::{
                authentication_user_details_service::AuthenticationUserDetailsService,
                user::User,
                user_details::UserDetails,
                username_not_found_error::UsernameNotFoundError,
            },
        },
        web::authentication::preauth::{
            pre_authenticated_authentication_provider::PreAuthenticatedAuthenticationProvider,
            pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken,
        },
    };

    struct StubAuthenticationUserDetailsService;

    #[async_trait]
    impl AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>
        for StubAuthenticationUserDetailsService
    {
        async fn load_user_details(
            &self,
            token: &PreAuthenticatedAuthenticationToken,
        ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
            Ok(Arc::new(User::new(
                token.get_name(),
                token.get_credentials(),
                AuthorityUtils::create_authority_list(["ROLE_USER"]),
            )))
        }
    }

    #[tokio::test]
    async fn pre_authenticated_authentication_provider_authenticates_token() {
        let provider = PreAuthenticatedAuthenticationProvider::new(
            Arc::new(StubAuthenticationUserDetailsService),
            Arc::new(AccountStatusUserDetailsChecker),
        );
        let token = PreAuthenticatedAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("external-credential")),
        );

        let result = provider.authenticate(&token).await.unwrap();
        assert!(result.is_authenticated());
        assert_eq!(result.get_name(), "alice");
        assert_eq!(result.authorities(), vec![String::from("ROLE_USER")]);
    }
}
