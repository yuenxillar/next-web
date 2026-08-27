use std::{
    any::TypeId,
    sync::{Arc, Mutex},
};

use next_web_core::async_trait;

use crate::{
    authentication::{
        account_status_user_details_exceptions::{
            bad_credentials, compromised_password, internal_authentication_service,
        },
        authentication_provider::AuthenticationProvider,
        dao::base_user_details_authentication_provider::BaseUserDetailsAuthenticationProviderSupport,
        password::CompromisedPasswordChecker,
    },
    core::{
        userdetails::{
            user_details_password_service::{
                NoopUserDetailsPasswordService, UserDetailsPasswordService,
            },
            UserDetails, UserDetailsService,
        },
        Authentication, AuthenticationError, AuthenticationErrorKind,
        UsernamePasswordAuthenticationToken,
    },
    crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
};

/// An AuthenticationProvider implementation that retrieves user details from a UserDetailsService.
pub struct DaoAuthenticationProvider {
    user_details_service: Arc<dyn UserDetailsService>,
    password_encoder: Arc<dyn PasswordEncoder>,
    user_not_found_encoded_password: Option<String>,
    user_details_password_service: Arc<dyn UserDetailsPasswordService>,
    compromised_password_checker: Option<Arc<dyn CompromisedPasswordChecker>>,
    support: BaseUserDetailsAuthenticationProviderSupport,
}

impl DaoAuthenticationProvider {
    /// The plaintext password used to perform PasswordEncoder.matches(CharSequence, String) on when the user is not found to avoid SEC-2056.
    const USER_NOT_FOUND_PASSWORD: &str = "userNotFoundPassword";

    pub fn new(user_details_service: Arc<dyn UserDetailsService>) -> Self {
        Self {
            user_details_service,
            password_encoder: Arc::new(BCryptPasswordEncoder),
            user_not_found_encoded_password: None,
            user_details_password_service: Arc::new(NoopUserDetailsPasswordService),
            compromised_password_checker: None,
            support: BaseUserDetailsAuthenticationProviderSupport::default(),
        }
    }

    /// Sets the PasswordEncoder instance to be used to encode and validate passwords.
    ///  If not set, the password will be compared using PasswordEncoderFactories.create_delegating_password_encoder()
    pub fn set_password_encoder(&mut self, password_encoder: Arc<dyn PasswordEncoder>) {
        self.password_encoder = password_encoder;
        self.user_not_found_encoded_password = None;
    }

    pub fn password_encoder(&self) -> &dyn PasswordEncoder {
        self.password_encoder.as_ref()
    }

    pub fn user_details_service(&self) -> &dyn UserDetailsService {
        self.user_details_service.as_ref()
    }

    pub fn set_user_details_password_service(
        &mut self,
        user_details_password_service: Arc<dyn UserDetailsPasswordService>,
    ) {
        self.user_details_password_service = user_details_password_service;
    }

    /// Sets the CompromisedPasswordChecker to be used before creating a successful authentication. Defaults to none.
    pub fn set_compromised_password_checker(
        &mut self,
        compromised_password_checker: Arc<dyn CompromisedPasswordChecker>,
    ) {
        self.compromised_password_checker = Some(compromised_password_checker);
    }

    pub fn support_mut(&mut self) -> &mut BaseUserDetailsAuthenticationProviderSupport {
        &mut self.support
    }

    async fn additional_authentication_checks(
        &self,
        user_details: Arc<dyn UserDetails>,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<(), AuthenticationError> {
        let Some(presented_password) = authentication.credentials() else {
            return Err(bad_credentials());
        };

        if let Some(password) = user_details.password() {
            if !self.password_encoder.matches(&presented_password, password) {
                return Err(bad_credentials());
            }
        }

        Ok(())
    }

    async fn retrieve_user(
        &self,
        username: &str,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        self.prepare_timing_attack_protection()?;

        match self
            .user_details_service
            .load_user_by_username(username.to_string())
            .await
        {
            Ok(user) => Ok(user),
            Err(error) => {
                self.mitigate_against_timing_attack(authentication);
                if self.support.is_hide_user_not_found_exceptions() {
                    Err(bad_credentials())
                } else {
                    Err(AuthenticationError::new(error.to_string()))
                }
            }
        }
    }

    fn prepare_timing_attack_protection(&mut self) -> Result<(), AuthenticationError> {
        if self.user_not_found_encoded_password.is_none() {
            self.user_not_found_encoded_password = Some(
                self.password_encoder
                    .encode(Self::USER_NOT_FOUND_PASSWORD)
                    .map_err(|error| {
                        AuthenticationError::with_kind(
                            error.to_string(),
                            AuthenticationErrorKind::InternalAuthentication,
                        )
                    })?,
            );
        }

        Ok(())
    }

    fn mitigate_against_timing_attack(&self, authentication: &UsernamePasswordAuthenticationToken) {
        let Some(credentials) = authentication.credentials() else {
            return;
        };
        let Some(encoded_password) = self.user_not_found_encoded_password.as_deref() else {
            return;
        };
        if let Some(presented_password) = credentials.as_ref().downcast_ref::<String>().as_deref() {
            self.password_encoder
                .matches(&presented_password, encoded_password);
        }
    }

    async fn maybe_upgrade_password(
        &self,
        user: Arc<dyn UserDetails>,
        presented_password: Option<String>,
    ) -> Arc<dyn UserDetails> {
        let Some(presented_password) = presented_password else {
            return user;
        };
        let existing_encoded_password = user.password().unwrap_or_default();
        if existing_encoded_password.is_empty()
            || !self
                .password_encoder
                .upgrade_encoding(&existing_encoded_password)
        {
            return user;
        }

        let Ok(new_password) = self.password_encoder.encode(&presented_password) else {
            return user;
        };
        self.user_details_password_service
            .update_password(user, Some(new_password))
            .await
    }

    fn check_compromised_password(
        &self,
        password: Option<&str>,
    ) -> Result<(), AuthenticationError> {
        let Some(checker) = &self.compromised_password_checker else {
            return Ok(());
        };
        if checker.check(password).is_compromised() {
            return Err(compromised_password(
                "The provided password is compromised, please change your password",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl AuthenticationProvider for DaoAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        // let Some(authentication) = authentication
        //     .as_any()
        //     .downcast_ref::<UsernamePasswordAuthenticationToken>()
        // else {
        //     return Err(AuthenticationError::new(
        //         "Only UsernamePasswordAuthenticationToken is supported",
        //     ));
        // };

        // let username = self.support.determine_username(authentication);
        // let cached_user = self.support.user_cache().get_user_from_cache(&username);
        // let mut cache_was_used = cached_user.is_some();
        // let mut user = if let Some(user) = cached_user {
        //     user
        // } else {
        //     self.retrieve_user(&username, authentication).await?
        // };

        // let check_result = self
        //     .support
        //     .perform_pre_authentication_checks(user.as_ref())
        //     .await;

        // if let Err(error) = check_result {
        //     if self.support.always_perform_additional_checks_on_user() {
        //         let _ = self
        //             .additional_authentication_checks(user.clone(), authentication)
        //             .await;
        //     }
        //     if !cache_was_used {
        //         return Err(error);
        //     }
        //     cache_was_used = false;
        //     user = self.retrieve_user(&username, authentication).await?;
        //     self.support
        //         .perform_pre_authentication_checks(user.as_ref())
        //         .await?;
        //     self.additional_authentication_checks(user.clone(), authentication)
        //         .await?;
        // } else {
        //     self.additional_authentication_checks(user.clone(), authentication)
        //         .await?;
        // }

        // self.support
        //     .perform_post_authentication_checks(user.as_ref())
        //     .await?;

        // if !cache_was_used {
        //     self.support
        //         .user_cache()
        //         .put_user_in_cache(username.clone(), user.clone());
        // }

        // let presented_password = authentication.get_credentials();
        // self.check_compromised_password(presented_password.as_deref())?;
        // let user = self
        //     .maybe_upgrade_password(user, presented_password.clone())
        //     .await;

        // self.support
        //     .create_success_authentication(user.username(), authentication, user.as_ref())
        //     .await
        todo!()
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<UsernamePasswordAuthenticationToken>()
    }
}
