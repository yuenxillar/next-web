use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::ArcSwap;
use tracing::debug;

use crate::{
    authentication::{
        dao::BaseUserDetailsAuthenticationProvider, password::CompromisedPasswordChecker,
        UsernamePasswordAuthenticationToken,
    },
    core::{
        userdetails::{
            NoopUserDetailsPasswordService, UserDetails, UserDetailsPasswordService,
            UserDetailsService,
        },
        Authentication, AuthenticationError, AuthenticationErrorKind,
    },
    crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
};

/// An AuthenticationProvider implementation that retrieves user details from a UserDetailsService.
pub struct DaoAuthenticationProvider {
    password_encoder: Arc<dyn PasswordEncoder>,
    user_not_found_encoded_password: ArcSwap<Option<String>>,
    user_details_service: Arc<dyn UserDetailsService>,
    user_details_password_service: Arc<dyn UserDetailsPasswordService>,
    compromised_password_checker: Option<Arc<dyn CompromisedPasswordChecker>>,

    base: BaseUserDetailsAuthenticationProvider,
}

impl DaoAuthenticationProvider {
    /// The plaintext password used to perform PasswordEncoder.matches(CharSequence, String) on when the user is not found to avoid SEC-2056.
    const USER_NOT_FOUND_PASSWORD: &str = "userNotFoundPassword";

    pub fn new(user_details_service: Arc<dyn UserDetailsService>) -> Self {
        Self {
            user_details_service,
            password_encoder: Arc::new(BCryptPasswordEncoder),
            user_not_found_encoded_password: ArcSwap::from_pointee(None),
            user_details_password_service: Arc::new(NoopUserDetailsPasswordService),
            compromised_password_checker: None,

            base: Default::default(),
        }
    }

    /// Sets the PasswordEncoder instance to be used to encode and validate passwords.
    ///  If not set, the password will be compared using PasswordEncoderFactories.create_delegating_password_encoder()
    pub fn set_password_encoder(&mut self, password_encoder: Arc<dyn PasswordEncoder>) {
        self.password_encoder = password_encoder;
        self.user_not_found_encoded_password.store(Arc::new(None));
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

    async fn additional_authentication_checks(
        &self,
        user_details: Arc<dyn UserDetails>,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<(), AuthenticationError> {
        let presented_password = match authentication.credentials() {
            Some(credentials) => credentials.to_string(),
            None => {
                debug!("Failed to authenticate since no credentials provided");
                return Err(AuthenticationError::with_kind(
                    self.messages.message_or_default(
                        "BaseUserDetailsAuthenticationProvider.badCredentials",
                        None,
                        "Bad credentials",
                    ),
                    AuthenticationErrorKind::BadCredentials,
                ));
            }
        };

        if let Some(password) = user_details.password() {
            if !self.password_encoder.matches(&presented_password, password) {
                debug!("Failed to authenticate since password does not match stored value");
                return Err(AuthenticationError::with_kind(
                    self.messages.message_or_default(
                        "BaseUserDetailsAuthenticationProvider.badCredentials",
                        None,
                        "Bad credentials",
                    ),
                    AuthenticationErrorKind::BadCredentials,
                ));
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
            .load_user_by_username(username)
            .await
        {
            Ok(user) => Ok(user),
            Err(err) => {
                if err.kind() == AuthenticationErrorKind::UsernameNotFound {
                    self.mitigate_against_timing_attack(authentication);
                }

                Err(err)
            }
        }
    }

    async fn create_success_authentication(
        &self,
        principal: Arc<dyn UserDetails>,
        authentication: &dyn Authentication,
        user: &dyn UserDetails,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let presented_password = match authentication.credentials() {
            Some(val) => val.to_string(),
            None => {
                return Err(AuthenticationError::new(
                    "Authentication.credentials() cannot be none",
                ))
            }
        };
        let is_password_compromised = self
            .compromised_password_checker
            .as_ref()
            .map(|checker| checker.check(Some(&presented_password)).is_compromised())
            .unwrap_or_default();

        if is_password_compromised {
            return Err(AuthenticationError::with_kind(
                "The provided password is compromised, please change your password",
                AuthenticationErrorKind::CompromisedPassword,
            ));
        }

        let existing_encoded_password = user.password();
        let upgrade_encoding = existing_encoded_password
            .as_ref()
            .map(|s| self.password_encoder.as_ref().upgrade_encoding(s))
            .unwrap_or_default();

        let mut _user = None;
        if upgrade_encoding {
            let new_password = self
                .password_encoder
                .as_ref()
                .encode(&presented_password)
                .map_err(|err| AuthenticationError::new(err.to_string()))?;
            _user = Some(
                self.user_details_password_service
                    .update_password(user, Some(new_password))
                    .await,
            );
        }

        self.base
            .create_success_authentication(
                principal,
                authentication,
                _user.as_deref().unwrap_or(user),
            )
            .await
    }

    fn prepare_timing_attack_protection(&self) -> Result<(), AuthenticationError> {
        if self.user_not_found_encoded_password.load().is_none() {
            let password = self
                .password_encoder
                .encode(Self::USER_NOT_FOUND_PASSWORD)
                .map_err(|error| {
                    AuthenticationError::with_kind(
                        error.to_string(),
                        AuthenticationErrorKind::InternalAuthentication,
                    )
                })?;
            self.user_not_found_encoded_password
                .store(Arc::new(Some(password)));
        }

        Ok(())
    }

    fn mitigate_against_timing_attack(&self, authentication: &UsernamePasswordAuthenticationToken) {
        let Some(credentials) = authentication.credentials() else {
            return;
        };

        match self.user_not_found_encoded_password.load().as_deref() {
            Some(password) => {
                let presented_password = credentials.to_string();
                self.password_encoder.matches(&presented_password, password);
            }
            None => return,
        }
    }

    // async fn maybe_upgrade_password(
    //     &self,
    //     user: Arc<dyn UserDetails>,
    //     presented_password: Option<String>,
    // ) -> Arc<dyn UserDetails> {
    //     let Some(presented_password) = presented_password else {
    //         return user;
    //     };
    //     let existing_encoded_password = user.password().unwrap_or_default();
    //     if existing_encoded_password.is_empty()
    //         || !self
    //             .password_encoder
    //             .upgrade_encoding(&existing_encoded_password)
    //     {
    //         return user;
    //     }

    //     let Ok(new_password) = self.password_encoder.encode(&presented_password) else {
    //         return user;
    //     };
    //     self.user_details_password_service
    //         .update_password(user, Some(new_password))
    //         .await
    // }

    // fn check_compromised_password(
    //     &self,
    //     password: Option<&str>,
    // ) -> Result<(), AuthenticationError> {
    //     let Some(checker) = &self.compromised_password_checker else {
    //         return Ok(());
    //     };
    //     if checker.check(password).is_compromised() {
    //         return Err(compromised_password(
    //             "The provided password is compromised, please change your password",
    //         ));
    //     }
    //     Ok(())
    // }
}

// #[async_trait]
// impl AuthenticationProvider for DaoAuthenticationProvider {
//     async fn authenticate(
//         &self,
//         authentication: &Arc<dyn Authentication>,
//     ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
//         // let Some(authentication) = authentication
//         //     .as_any()
//         //     .downcast_ref::<UsernamePasswordAuthenticationToken>()
//         // else {
//         //     return Err(AuthenticationError::new(
//         //         "Only UsernamePasswordAuthenticationToken is supported",
//         //     ));
//         // };

//         // let username = self.support.determine_username(authentication);
//         // let cached_user = self.support.user_cache().get_user_from_cache(&username);
//         // let mut cache_was_used = cached_user.is_some();
//         // let mut user = if let Some(user) = cached_user {
//         //     user
//         // } else {
//         //     self.retrieve_user(&username, authentication).await?
//         // };

//         // let check_result = self
//         //     .support
//         //     .perform_pre_authentication_checks(user.as_ref())
//         //     .await;

//         // if let Err(error) = check_result {
//         //     if self.support.always_perform_additional_checks_on_user() {
//         //         let _ = self
//         //             .additional_authentication_checks(user.clone(), authentication)
//         //             .await;
//         //     }
//         //     if !cache_was_used {
//         //         return Err(error);
//         //     }
//         //     cache_was_used = false;
//         //     user = self.retrieve_user(&username, authentication).await?;
//         //     self.support
//         //         .perform_pre_authentication_checks(user.as_ref())
//         //         .await?;
//         //     self.additional_authentication_checks(user.clone(), authentication)
//         //         .await?;
//         // } else {
//         //     self.additional_authentication_checks(user.clone(), authentication)
//         //         .await?;
//         // }

//         // self.support
//         //     .perform_post_authentication_checks(user.as_ref())
//         //     .await?;

//         // if !cache_was_used {
//         //     self.support
//         //         .user_cache()
//         //         .put_user_in_cache(username.clone(), user.clone());
//         // }

//         // let presented_password = authentication.get_credentials();
//         // self.check_compromised_password(presented_password.as_deref())?;
//         // let user = self
//         //     .maybe_upgrade_password(user, presented_password.clone())
//         //     .await;

//         // self.support
//         //     .create_success_authentication(user.username(), authentication, user.as_ref())
//         //     .await
//         todo!()
//     }

//     fn supports(&self, authentication: TypeId) -> bool {
//         authentication == TypeId::of::<UsernamePasswordAuthenticationToken>()
//     }
// }

impl Deref for DaoAuthenticationProvider {
    type Target = BaseUserDetailsAuthenticationProvider;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for DaoAuthenticationProvider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
