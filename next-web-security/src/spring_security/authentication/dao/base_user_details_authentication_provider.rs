use std::sync::Arc;

use crate::{
    authentication::{
        account_status_user_details_checker::AccountStatusUserDetailsChecker,
        account_status_user_details_exceptions::credentials_expired,
    },
    core::{
        authentication_error::AuthenticationError,
        authority_mapping::{GrantedAuthoritiesMapper, NullAuthoritiesMapper},
        authority_utils::AuthorityUtils,
        user_cache::{NullUserCache, UserCache},
        userdetails::{UserDetails, UserDetailsChecker},
        Authentication, UsernamePasswordAuthenticationToken,
    },
};

#[derive(Clone)]
pub struct BaseUserDetailsAuthenticationProviderSupport {
    user_cache: Arc<dyn UserCache>,
    force_principal_as_string: bool,
    hide_user_not_found_exceptions: bool,
    pre_authentication_checks: Arc<dyn UserDetailsChecker>,
    post_authentication_checks: Arc<dyn UserDetailsChecker>,
    always_perform_additional_checks_on_user: bool,
    authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
}

impl Default for BaseUserDetailsAuthenticationProviderSupport {
    fn default() -> Self {
        Self {
            user_cache: Arc::new(NullUserCache),
            force_principal_as_string: false,
            hide_user_not_found_exceptions: true,
            pre_authentication_checks: Arc::new(AccountStatusUserDetailsChecker),
            post_authentication_checks: Arc::new(CredentialsNonExpiredChecker),
            always_perform_additional_checks_on_user: true,
            authorities_mapper: Arc::new(NullAuthoritiesMapper),
        }
    }
}

impl BaseUserDetailsAuthenticationProviderSupport {
    pub fn determine_username(&self, authentication: &dyn Authentication) -> String {
        let username = authentication.name();
        if username.is_empty() {
            "NONE_PROVIDED"
        } else {
            username
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

    pub async fn create_success_authentication(
        &self,
        principal: impl Into<String>,
        authentication: &dyn Authentication,
        user: &dyn UserDetails,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let mut authority_names = Vec::new();
        for authority in user.authorities() {
            if let Some(authority) = authority.authority() {
                authority_names.push(authority);
            }
        }

        let mapped = self
            .authorities_mapper
            .map_authorities(AuthorityUtils::create_authority_list(authority_names));
        let mut result = UsernamePasswordAuthenticationToken::authenticated(
            principal.into(),
            authentication.credentials(),
            mapped,
        );
        result.set_details_value(authentication.details_value());
        Ok(Arc::new(result))
    }
}

#[derive(Clone, Default)]
struct CredentialsNonExpiredChecker;

impl UserDetailsChecker for CredentialsNonExpiredChecker {
    fn check(&self, to_check: &dyn UserDetails) -> Result<(), AuthenticationError> {
        if !to_check.is_credentials_non_expired() {
            return Err(credentials_expired());
        }
        Ok(())
    }
}
