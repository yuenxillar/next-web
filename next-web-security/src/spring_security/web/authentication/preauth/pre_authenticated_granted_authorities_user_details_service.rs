use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{
    authority::GrantedAuthoritiesContainer,
    userdetails::{AuthenticationUserDetailsService, User, UserDetails},
    Authentication, AuthenticationError, AuthenticationErrorKind, GrantedAuthority,
};

use super::{
    pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken,
    pre_authenticated_granted_authorities_web_authentication_details::PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails,
};

/// This AuthenticationUserDetailsService implementation creates a UserDetails object based solely on the information contained in the given PreAuthenticatedAuthenticationToken.
/// The user name is set to the name as returned by PreAuthenticatedAuthenticationToken.getName(), the password is set to a fixed dummy value (it will not be used by
/// the PreAuthenticatedAuthenticationProvider anyway), and the Granted Authorities are retrieved from the details object as returned by  PreAuthenticatedAuthenticationToken.details().
///
/// The details object as returned by PreAuthenticatedAuthenticationToken.getDetails() must implement the GrantedAuthoritiesContainer interface for this implementation to work.
#[derive(Clone, Default)]
pub struct PreAuthenticatedGrantedAuthoritiesUserDetailsService;

impl PreAuthenticatedGrantedAuthoritiesUserDetailsService {
    /// Creates the final UserDetails object. Can be overridden to customize the contents.

    pub fn create_user_details(
        &self,
        token: &dyn Authentication,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Arc<dyn UserDetails> {
        Arc::new(User::with_flags(
            token.name().as_ref(),
            Some(String::from("N/A")),
            true,
            true,
            true,
            true,
            authorities.to_vec(),
        ))
    }
}

#[async_trait]
impl AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>
    for PreAuthenticatedGrantedAuthoritiesUserDetailsService
{
    /// Get a UserDetails object based on the user name contained in the given token, and the
    /// GrantedAuthorities as returned by the GrantedAuthoritiesContainer implementation as returned by the token.getDetails() method.
    async fn load_user_details(
        &self,
        token: &PreAuthenticatedAuthenticationToken,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        let details = token
            .details()
            .and_then(|value| {
                value
                    .as_any()
                    .downcast_ref::<PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails>()
            })
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "token.get_details() must contain GrantedAuthoritiesContainer details",
                    AuthenticationErrorKind::UsernameNotFound,
                )
            })?;

        let authorities = details.granted_authorities();
        Ok(self.create_user_details(token, authorities))
    }
}
