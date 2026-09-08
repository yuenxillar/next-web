use std::{
    any::{Any, TypeId},
    collections::HashSet,
    ops::Deref,
    sync::Arc,
};

use next_web_context::support::MessageSourceAccessor;
use next_web_core::async_trait;
use tracing::debug;

use crate::{
    authentication::{
        account_status_user_details_checker::AccountStatusUserDetailsChecker,
        account_status_user_details_exceptions::credentials_expired, AuthenticationProvider,
        UsernamePasswordAuthenticationToken,
    },
    core::{
        authority::{
            mapping::{GrantedAuthoritiesMapper, NullAuthoritiesMapper},
            AuthorityUtils, FactorGrantedAuthority,
        },
        userdetails::{cache::NullUserCache, UserCache, UserDetails, UserDetailsChecker},
        Authentication, AuthenticationError, NextSecurityMessageSource,
    },
    web::authentication::AuthPrincipal,
};

/// A base AuthenticationProvider that allows subclasses to override and work with UserDetails objects.
/// The class is designed to respond to UsernamePasswordAuthenticationToken authentication requests.
///
/// Upon successful validation, a UsernamePasswordAuthenticationToken will be created and returned to the caller.
/// The token will include as its principal either a String representation of the username, or the UserDetails
/// that was returned from the authentication repository. Using String is appropriate if a container adapter is being used,
/// as it expects String representations of the username. Using UserDetails is appropriate if you require access to additional
/// properties of the authenticated user, such as email addresses, human-friendly names etc. As container adapters are not recommended
/// to be used, and UserDetails implementations provide additional flexibility, by default a UserDetails is returned. To override this default, set the setForcePrincipalAsString to true.
///
/// Caching is handled by storing the UserDetails object being placed in the UserCache. This ensures that subsequent
/// requests with the same username can be validated without needing to query the UserDetailsService. It should be noted that if a user
/// appears to present an incorrect password, the UserDetailsService will be queried to confirm the most up-to-date password was
/// used for comparison. Caching is only likely to be required for stateless applications. In a normal web application, for example, the
/// SecurityContext is stored in the user's session and the user isn't reauthenticated on each request. The default cache implementation is therefore NullUserCache.
#[derive(Clone)]
pub struct BaseUserDetailsAuthenticationProvider {
    pub(super) messages: MessageSourceAccessor,
    user_cache: Arc<dyn UserCache>,
    force_principal_as_string: bool,
    hide_user_not_found_exceptions: bool,
    pre_authentication_checks: Arc<dyn UserDetailsChecker>,
    post_authentication_checks: Arc<dyn UserDetailsChecker>,
    always_perform_additional_checks_on_user: bool,
    authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
}

impl BaseUserDetailsAuthenticationProvider {
    pub fn determine_username(&self, authentication: &dyn Authentication) -> String {
        let username = authentication.name();
        if username.is_empty() {
            "NONE_PROVIDED".to_owned()
        } else {
            username.into_owned()
        }
    }

    pub fn user_cache(&self) -> Arc<dyn UserCache> {
        self.user_cache.clone()
    }

    pub fn set_user_cache(&mut self, user_cache: Arc<dyn UserCache>) {
        self.user_cache = user_cache;
    }

    pub fn is_force_principal_as_string(&self) -> bool {
        self.force_principal_as_string
    }

    pub fn set_force_principal_as_string(&mut self, force_principal_as_string: bool) {
        self.force_principal_as_string = force_principal_as_string;
    }

    pub fn is_hide_user_not_found_exceptions(&self) -> bool {
        self.hide_user_not_found_exceptions
    }

    pub fn set_hide_user_not_found_exceptions(&mut self, hide_user_not_found_exceptions: bool) {
        self.hide_user_not_found_exceptions = hide_user_not_found_exceptions;
    }

    pub fn set_pre_authentication_checks(
        &mut self,
        pre_authentication_checks: Arc<dyn UserDetailsChecker>,
    ) {
        self.pre_authentication_checks = pre_authentication_checks;
    }

    pub fn set_post_authentication_checks(
        &mut self,
        post_authentication_checks: Arc<dyn UserDetailsChecker>,
    ) {
        self.post_authentication_checks = post_authentication_checks;
    }

    pub fn set_always_perform_additional_checks_on_user(&mut self, value: bool) {
        self.always_perform_additional_checks_on_user = value;
    }

    pub fn set_authorities_mapper(
        &mut self,
        authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    ) {
        self.authorities_mapper = authorities_mapper;
    }

    pub async fn perform_pre_authentication_checks(
        &self,
        user: &dyn UserDetails,
    ) -> Result<(), AuthenticationError> {
        self.pre_authentication_checks.check(user).await
    }

    pub fn always_perform_additional_checks_on_user(&self) -> bool {
        self.always_perform_additional_checks_on_user
    }

    pub async fn perform_post_authentication_checks(
        &self,
        user: &dyn UserDetails,
    ) -> Result<(), AuthenticationError> {
        self.post_authentication_checks.check(user).await
    }
}

impl Default for BaseUserDetailsAuthenticationProvider {
    fn default() -> Self {
        Self {
            messages: NextSecurityMessageSource::get_accessor(),
            user_cache: Arc::new(NullUserCache),
            force_principal_as_string: false,
            hide_user_not_found_exceptions: true,
            pre_authentication_checks: Arc::new(DefaultPreAuthenticationChecks::default()),
            post_authentication_checks: Arc::new(DefaultPostAuthenticationChecks::default()),
            always_perform_additional_checks_on_user: true,
            authorities_mapper: Arc::new(NullAuthoritiesMapper),
        }
    }
}

#[derive(Clone, Default)]
struct DefaultPostAuthenticationChecks;

impl UserDetailsChecker for DefaultPostAuthenticationChecks {
    fn check(&self, to_check: &dyn UserDetails) -> Result<(), AuthenticationError> {
        if !to_check.is_credentials_non_expired() {
            return Err(credentials_expired());
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
struct DefaultPreAuthenticationChecks {}

#[async_trait]
pub trait BaseUserDetailsAuthenticationProviderExt
where
    Self: Deref<Target = BaseUserDetailsAuthenticationProvider>,
{
    /// Allows subclasses to perform any additional checks of a returned (or cached) UserDetails for a given authentication request.
    /// Generally a subclass will at least compare the Authentication.getCredentials() with a UserDetails.getPassword().
    /// If custom logic is needed to compare additional properties of UserDetails and/or UsernamePasswordAuthenticationToken,
    /// these should also appear in this method.
    async fn additional_authentication_checks(
        &self,
        user_details: Arc<dyn UserDetails>,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<(), AuthenticationError>;

    async fn retrieve_user(
        &self,
        username: &str,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError>;

    async fn create_success_authentication(
        &self,
        principal: AuthPrincipal,
        authentication: &dyn Authentication,
        user: &dyn UserDetails,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        // Ensure we return the original credentials the user supplied,
        // so subsequent attempts are successful even with encoded passwords.
        // Also ensure we return the original getDetails(), so that future
        // authentication events after cache expiry contain the details
        let mut authorities = self.authorities_mapper.map_authorities(user.authorities());
        authorities.push(Arc::new(FactorGrantedAuthority::from_authority(
            FactorGrantedAuthority::PASSWORD_AUTHORITY,
        )));
        let mut seen = HashSet::new();
        authorities.retain(|a| {
            a.authority()
                .map(|s| seen.insert(s.to_string()))
                .unwrap_or(false)
        });

        let mut result = UsernamePasswordAuthenticationToken::authenticated(
            principal,
            authentication.credentials().cloned(),
            authorities,
        );
        result.set_details(authentication.details().cloned());
        debug!("Authenticated user");

        Ok(Arc::new(result))
    }
}

#[async_trait]
impl<T> AuthenticationProvider for T
where
    T: BaseUserDetailsAuthenticationProviderExt,
    T: Deref<Target = BaseUserDetailsAuthenticationProvider>,
    T: Send + Sync,
{
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let Some(authentication) = (authentication.as_ref() as &dyn Any)
            .downcast_ref::<UsernamePasswordAuthenticationToken>()
        else {
            return Err(AuthenticationError::new(self.messages.message_or_default(
                "BaseUserDetailsAuthenticationProvider.onlySupports",
                None,
                "Only UsernamePasswordAuthenticationToken is supported",
            )));
        };

        let username = self.determine_username(authentication);
        let user = self.user_cache.get_user_from_cache(&username).await;
        let mut cache_was_used = user.is_some();
        let mut user = if let Some(user) = user {
            user
        } else {
            self.retrieve_user(&username, authentication).await?
        };

        let check_result = self.perform_pre_authentication_checks(user.as_ref()).await;

        if let Err(error) = check_result {
            if self.always_perform_additional_checks_on_user() {
                let _ = self
                    .additional_authentication_checks(user.clone(), authentication)
                    .await;
            }
            if !cache_was_used {
                return Err(error);
            }
            cache_was_used = false;
            user = self.retrieve_user(&username, authentication).await?;
            self.perform_pre_authentication_checks(user.as_ref())
                .await?;
            self.additional_authentication_checks(user.clone(), authentication)
                .await?;
        } else {
            self.additional_authentication_checks(user.clone(), authentication)
                .await?;
        }

        self.perform_post_authentication_checks(user.as_ref())
            .await?;

        if !cache_was_used {
            self.user_cache
                .put_user_in_cache(username.clone(), user.clone());
        }

        let presented_password = authentication.get_credentials();
        self.check_compromised_password(presented_password.as_deref())?;
        let user = self
            .maybe_upgrade_password(user, presented_password.clone())
            .await;

        self.create_success_authentication(user.username(), authentication, user.as_ref())
            .await
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<UsernamePasswordAuthenticationToken>()
    }
}
