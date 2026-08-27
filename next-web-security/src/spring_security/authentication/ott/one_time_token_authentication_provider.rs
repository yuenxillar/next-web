use std::any::Any;
use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::{
        authentication_provider::AuthenticationProvider,
        ott::{
            invalid_one_time_token_exception::invalid_one_time_token,
            one_time_token::OneTimeToken,
            one_time_token_authentication::OneTimeTokenAuthentication,
            one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
            one_time_token_service::OneTimeTokenService,
        },
    },
    core::{
        authority::FactorGrantedAuthority,
        userdetails::UserDetailsService,
        Authentication, AuthenticationError, AuthenticationErrorKind,
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
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let Some(token) = (authentication.as_ref() as &dyn Any)
            .downcast_ref::<OneTimeTokenAuthenticationToken>()
        else {
            return Ok(None);
        };

        let consumed = self
            .one_time_token_service
            .consume(token)
            .ok_or_else(|| invalid_one_time_token("Invalid one-time token"))?;

        let user = self
            .user_details_service
            .load_user_by_username(consumed.username())
            .await
            .map_err(|_| {
                AuthenticationError::with_kind(
                    "Failed to authenticate the one-time token",
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        let mut authorities: Vec<Arc<dyn crate::core::GrantedAuthority>> = user
            .authorities()
            .iter()
            .cloned()
            .collect();
        authorities.push(Arc::new(FactorGrantedAuthority::from_authority(
            FactorGrantedAuthority::OTT_AUTHORITY,
        )));

        let mut result = OneTimeTokenAuthentication::new(user.username().to_string(), authorities);
        result.set_details_value(authentication.details().cloned());

        Ok(Some(Arc::new(result)))
    }

    fn supports(&self, authentication: std::any::TypeId) -> bool {
        authentication == std::any::TypeId::of::<OneTimeTokenAuthenticationToken>()
    }
}
