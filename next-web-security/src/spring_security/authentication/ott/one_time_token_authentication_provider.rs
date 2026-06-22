use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::{
        authentication_provider::AuthenticationProvider,
        ott::{
            invalid_one_time_token_exception::invalid_one_time_token, one_time_token::OneTimeToken,
            one_time_token_authentication::OneTimeTokenAuthentication,
            one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
            one_time_token_service::OneTimeTokenService,
        },
    },
    core::{
        authentication_error::{AuthenticationError, AuthenticationErrorKind},
        authority_utils::AuthorityUtils,
        factor_granted_authority::FactorGrantedAuthority,
        userdetails::user_details_service::UserDetailsService,
        Authentication,
    },
};

pub struct OneTimeTokenAuthenticationProvider {
    one_time_token_service: Arc<dyn OneTimeTokenService>,
    user_details_service: Arc<dyn UserDetailsService>,
}

impl OneTimeTokenAuthenticationProvider {
    pub fn new(
        one_time_token_service: Arc<dyn OneTimeTokenService>,
        user_details_service: Arc<dyn UserDetailsService>,
    ) -> Self {
        Self {
            one_time_token_service,
            user_details_service,
        }
    }
}

#[async_trait]
impl AuthenticationProvider for OneTimeTokenAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let Some(token) = authentication
            .as_any()
            .downcast_ref::<OneTimeTokenAuthenticationToken>()
        else {
            return Err(AuthenticationError::new(
                "Only OneTimeTokenAuthenticationToken is supported",
            ));
        };

        let consumed = self
            .one_time_token_service
            .consume(token)
            .ok_or_else(|| invalid_one_time_token("Invalid token"))?;

        let user = self
            .user_details_service
            .load_user_by_username(consumed.username().to_string())
            .await
            .map_err(|_| {
                AuthenticationError::with_kind(
                    "Failed to authenticate the one-time token",
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        let mut authorities = user
            .get_authorities()
            .await
            .into_iter()
            .filter_map(|authority| futures::executor::block_on(authority.get_authority()))
            .collect::<Vec<_>>();
        authorities.push(FactorGrantedAuthority::OTT_AUTHORITY.to_string());

        let mut result = OneTimeTokenAuthentication::new(
            user.get_username().await,
            AuthorityUtils::create_authority_list(authorities),
        );
        result.set_details_value(authentication.get_details_value());
        Ok(Arc::new(result))
    }

    fn supports(&self, authentication: &str) -> bool {
        authentication == std::any::type_name::<OneTimeTokenAuthenticationToken>()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        authentication::{
            authentication_provider::AuthenticationProvider,
            ott::{
                generate_one_time_token_request::GenerateOneTimeTokenRequest,
                in_memory_one_time_token_service::InMemoryOneTimeTokenService,
                one_time_token::OneTimeToken,
                one_time_token_authentication_provider::OneTimeTokenAuthenticationProvider,
                one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
                one_time_token_service::OneTimeTokenService,
            },
        },
        core::{
            authority_utils::AuthorityUtils,
            factor_granted_authority::FactorGrantedAuthority,
            userdetails::{
                map_user_details_service::MapUserDetailsService, user::User,
                user_details::UserDetails,
            },
        },
    };

    #[tokio::test]
    async fn provider_authenticates_valid_one_time_token_and_adds_factor_authority() {
        let token_service = Arc::new(InMemoryOneTimeTokenService::new());
        let user_details_service = Arc::new(MapUserDetailsService::new([Arc::new(User::new(
            "alice",
            Some(String::from("password")),
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        ))
            as Arc<dyn UserDetails>]));
        let provider =
            OneTimeTokenAuthenticationProvider::new(token_service.clone(), user_details_service);
        let generated = token_service.generate(GenerateOneTimeTokenRequest::new("alice"));
        let authentication = OneTimeTokenAuthenticationToken::new(generated.token_value());

        let result = provider.authenticate(&authentication).await.unwrap();

        assert!(result.is_authenticated());
        assert_eq!(result.get_name(), "alice");
        assert_eq!(
            result.authorities(),
            vec![
                String::from("ROLE_USER"),
                FactorGrantedAuthority::OTT_AUTHORITY.to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn provider_rejects_invalid_one_time_token() {
        let provider = OneTimeTokenAuthenticationProvider::new(
            Arc::new(InMemoryOneTimeTokenService::new()),
            Arc::new(MapUserDetailsService::default()),
        );
        let authentication = OneTimeTokenAuthenticationToken::new("missing");

        let error = match provider.authenticate(&authentication).await {
            Ok(_) => panic!("expected invalid token to be rejected"),
            Err(error) => error,
        };

        assert_eq!(
            error.kind(),
            crate::core::authentication_error::AuthenticationErrorKind::BadCredentials
        );
    }
}
