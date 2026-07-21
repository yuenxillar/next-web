use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{
    granted_authorities_container::GrantedAuthoritiesContainer,
    userdetails::{
        authentication_user_details_service::AuthenticationUserDetailsService, user::User,
        username_not_found_error::UsernameNotFoundError, UserDetails,
    },
    Authentication,
};

use super::{
    pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken,
    pre_authenticated_granted_authorities_web_authentication_details::PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails,
};

#[derive(Clone, Default)]
pub struct PreAuthenticatedGrantedAuthoritiesUserDetailsService;

#[async_trait]
impl AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>
    for PreAuthenticatedGrantedAuthoritiesUserDetailsService
{
    async fn load_user_details(
        &self,
        token: &PreAuthenticatedAuthenticationToken,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
        let details = token
            .get_details_ref()
            .and_then(|value| {
                value.as_ref_object::<PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails>()
            })
            .ok_or_else(|| {
                UsernameNotFoundError(String::from(
                    "token.get_details() must contain GrantedAuthoritiesContainer details",
                ))
            })?;

        let username = token.get_name();
        Ok(Arc::new(User::with_flags(
            username,
            Some(String::from("N/A")),
            true,
            true,
            true,
            true,
            details.granted_authorities(),
        )))
    }
}
