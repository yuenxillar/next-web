use std::{any::TypeId, collections::HashSet, sync::Arc};

use next_web_core::async_trait;
use tracing::debug;

use crate::{
    authentication::{AccountStatusUserDetailsChecker, AuthenticationProvider},
    core::{
        userdetails::{AuthenticationUserDetailsService, UserDetailsChecker},
        Authentication, AuthenticationError, AuthenticationErrorKind, GrantedAuthority,
    },
    web::authentication::preauth::PreAuthenticatedAuthenticationToken,
};

/// Processes a pre-authenticated authentication request. The request will typically originate from a BasePreAuthenticatedProcessingFilter.
///
/// This authentication provider will not perform any checks on authentication requests, as they should already be pre-authenticated.
///  However, the AuthenticationUserDetailsService implementation may still throw a UsernameNotFoundException, for example.
pub struct PreAuthenticatedAuthenticationProvider {
    pre_authenticated_user_details_service:
        Option<Arc<dyn AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>>>,
    user_details_checker: Arc<dyn UserDetailsChecker>,
    granted_authorities: Vec<Arc<dyn GrantedAuthority>>,
    error_when_token_rejected: bool,
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
            pre_authenticated_user_details_service: Some(pre_authenticated_user_details_service),
            user_details_checker,
            granted_authorities: Vec::new(),
            error_when_token_rejected: false,
            order: -1,
        }
    }

    /// Sets the `AuthenticationUserDetailsService` used to load the `UserDetails` for the
    /// authenticated user.
    pub fn set_pre_authenticated_user_details_service(
        &mut self,
        pre_authenticated_user_details_service: Arc<
            dyn AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>,
        >,
    ) {
        self.pre_authenticated_user_details_service = Some(pre_authenticated_user_details_service);
    }

    /// Sets the strategy used to validate the loaded `UserDetails` object for the user.
    /// Defaults to an `AccountStatusUserDetailsChecker`.
    pub fn set_user_details_checker(&mut self, user_details_checker: Arc<dyn UserDetailsChecker>) {
        self.user_details_checker = user_details_checker;
    }

    /// If `true`, the provider rejects an invalid authentication request (with a null
    /// principal or credentials) with an error instead of returning `None`.
    pub fn set_error_when_token_rejected(&mut self, value: bool) {
        self.error_when_token_rejected = value;
    }

    /// Sets the authorities this provider should grant once authentication completes.
    pub fn set_granted_authorities(&mut self, authorities: Vec<Arc<dyn GrantedAuthority>>) {
        self.granted_authorities = authorities;
    }

    /// Returns the order of this provider.
    pub fn order(&self) -> i32 {
        self.order
    }

    /// Sets the order of this provider.
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
        let Some(authentication) = (authentication.as_ref() as &dyn std::any::Any)
            .downcast_ref::<PreAuthenticatedAuthenticationToken>()
        else {
            return Ok(None);
        };
        debug!(
            "PreAuthenticated authentication request: {}",
            authentication,
        );

        if authentication.principal().is_none() {
            debug!("No pre-authenticated principal found in request.");
            if self.error_when_token_rejected {
                return Err(AuthenticationError::with_kind(
                    "No pre-authenticated principal found in request.",
                    AuthenticationErrorKind::BadCredentials,
                ));
            }
            return Ok(None);
        }

        if authentication.credentials().is_none() {
            debug!("No pre-authenticated credentials found in request.");
            if self.error_when_token_rejected {
                return Err(AuthenticationError::with_kind(
                    "No pre-authenticated credentials found in request.",
                    AuthenticationErrorKind::BadCredentials,
                ));
            }
            return Ok(None);
        }

        let user_details_service = match self.pre_authenticated_user_details_service.as_ref() {
            Some(user_details_service) => user_details_service,
            None => return Ok(None),
        };

        let user_details = user_details_service
            .load_user_details(authentication)
            .await?;

        self.user_details_checker.check(user_details.as_ref())?;

        let value = user_details.authorities();
        let mut seen = HashSet::with_capacity(value.len());
        let mut authorities: Vec<Arc<dyn GrantedAuthority>> = Vec::with_capacity(value.len());

        for authority in value.iter() {
            let key = authority.authority().unwrap_or("");
            if seen.insert(key) {
                authorities.push(Arc::clone(authority));
            }
        }
        authorities.extend(self.granted_authorities.iter().cloned());

        let mut result = PreAuthenticatedAuthenticationToken::with_authorities(
            user_details,
            authentication.credentials().cloned(),
            authorities,
        );
        result.set_details(authentication.details().cloned());

        return Ok(Some(Arc::new(result)));
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<PreAuthenticatedAuthenticationToken>()
    }
}

impl Default for PreAuthenticatedAuthenticationProvider {
    fn default() -> Self {
        Self {
            pre_authenticated_user_details_service: None,
            user_details_checker: Arc::new(AccountStatusUserDetailsChecker::default()),
            granted_authorities: Vec::new(),
            error_when_token_rejected: false,
            order: -1,
        }
    }
}
