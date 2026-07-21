use std::{any::TypeId, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{
        authentication_error::AuthenticationError,
        authority_utils::AuthorityUtils,
        userdetails::{
            authentication_user_details_service::AuthenticationUserDetailsService,
            UserDetailsChecker,
        },
        Authentication,
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
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let Some(authentication) =
            authentication.downcast_ref::<PreAuthenticatedAuthenticationToken>()
        else {
            return Err(AuthenticationError::new(
                "Only PreAuthenticatedAuthenticationToken is supported",
            ));
        };

        if authentication.principal().is_none() {
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
        self.user_details_checker
            .check(user_details.as_ref())
            .await?;

        let mut authorities = Vec::new();
        for authority in user_details.authorities() {
            if let Some(name) = authority.authority() {
                authorities.push(name.to_string());
            }
        }
        authorities.extend(self.granted_authorities.iter().cloned());

        let mut result = PreAuthenticatedAuthenticationToken::authenticated(
            user_details.username(),
            authentication.get_credentials(),
            AuthorityUtils::create_authority_list(authorities),
        );
        result.set_details_value(authentication.get_details_value());
        Ok(Some(Arc::new(result)))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<PreAuthenticatedAuthenticationToken>()
    }
}
