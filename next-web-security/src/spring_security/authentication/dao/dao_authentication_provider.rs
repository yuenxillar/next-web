use std::sync::{Arc, Mutex};

use next_web_core::async_trait;

use crate::{
    authentication::{
        account_status_user_details_exceptions::{
            bad_credentials, compromised_password, internal_authentication_service,
        },
        authentication_provider::AuthenticationProvider,
        dao::abstract_user_details_authentication_provider::AbstractUserDetailsAuthenticationProviderSupport,
        password::compromised_password_checker::CompromisedPasswordChecker,
    },
    core::{
        authentication::Authentication,
        authentication_error::AuthenticationError,
        userdetails::{
            user_details::UserDetails,
            user_details_password_service::{NoopUserDetailsPasswordService, UserDetailsPasswordService},
            user_details_service::UserDetailsService,
        },
        username_password_authentication_token::UsernamePasswordAuthenticationToken,
    },
    crypto::{bcrypt::BCryptPasswordEncoder, password::password_encoder::PasswordEncoder},
};

pub struct DaoAuthenticationProvider {
    user_details_service: Arc<dyn UserDetailsService>,
    password_encoder: Arc<dyn PasswordEncoder>,
    user_not_found_encoded_password: Mutex<Option<String>>,
    user_details_password_service: Arc<dyn UserDetailsPasswordService>,
    compromised_password_checker: Option<Arc<dyn CompromisedPasswordChecker>>,
    support: AbstractUserDetailsAuthenticationProviderSupport,
}

impl DaoAuthenticationProvider {
    pub fn new(user_details_service: Arc<dyn UserDetailsService>) -> Self {
        Self {
            user_details_service,
            password_encoder: Arc::new(BCryptPasswordEncoder),
            user_not_found_encoded_password: Mutex::new(None),
            user_details_password_service: Arc::new(NoopUserDetailsPasswordService),
            compromised_password_checker: None,
            support: AbstractUserDetailsAuthenticationProviderSupport::default(),
        }
    }

    pub fn set_password_encoder(&mut self, password_encoder: Arc<dyn PasswordEncoder>) {
        self.password_encoder = password_encoder;
        if let Ok(mut encoded_password) = self.user_not_found_encoded_password.lock() {
            *encoded_password = None;
        }
    }

    pub fn set_user_details_password_service(
        &mut self,
        user_details_password_service: Arc<dyn UserDetailsPasswordService>,
    ) {
        self.user_details_password_service = user_details_password_service;
    }

    pub fn set_compromised_password_checker(
        &mut self,
        compromised_password_checker: Arc<dyn CompromisedPasswordChecker>,
    ) {
        self.compromised_password_checker = Some(compromised_password_checker);
    }

    pub fn support_mut(&mut self) -> &mut AbstractUserDetailsAuthenticationProviderSupport {
        &mut self.support
    }

    async fn additional_authentication_checks(
        &self,
        user_details: Arc<dyn UserDetails>,
        authentication: &UsernamePasswordAuthenticationToken,
    ) -> Result<(), AuthenticationError> {
        let Some(presented_password) = authentication.get_credentials() else {
            return Err(bad_credentials());
        };

        if !self
            .password_encoder
            .matches(&presented_password, &user_details.get_password().await)
        {
            return Err(bad_credentials());
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

    fn prepare_timing_attack_protection(&self) -> Result<(), AuthenticationError> {
        let mut encoded_password = self
            .user_not_found_encoded_password
            .lock()
            .map_err(|_| internal_authentication_service("User-not-found password cache was poisoned"))?;
        if encoded_password.is_none() {
            *encoded_password = Some(
                self.password_encoder
                    .encode("userNotFoundPassword")
                    .map_err(|error| internal_authentication_service(error.to_string()))?,
            );
        }
        Ok(())
    }

    fn mitigate_against_timing_attack(
        &self,
        authentication: &UsernamePasswordAuthenticationToken,
    ) {
        let Some(credentials) = authentication.get_credentials() else {
            return;
        };
        let Ok(encoded_password) = self.user_not_found_encoded_password.lock() else {
            return;
        };
        if let Some(encoded_password) = encoded_password.as_deref() {
            let _ = self.password_encoder.matches(&credentials, encoded_password);
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
        let existing_encoded_password = user.get_password().await;
        if existing_encoded_password.is_empty()
            || !self.password_encoder.upgrade_encoding(&existing_encoded_password)
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
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let Some(authentication) = authentication
            .as_any()
            .downcast_ref::<UsernamePasswordAuthenticationToken>()
        else {
            return Err(AuthenticationError::new(
                "Only UsernamePasswordAuthenticationToken is supported",
            ));
        };

        let username = self.support.determine_username(authentication);
        let cached_user = self.support.user_cache().get_user_from_cache(&username);
        let mut cache_was_used = cached_user.is_some();
        let mut user = if let Some(user) = cached_user {
            user
        } else {
            self.retrieve_user(&username, authentication).await?
        };

        let check_result = self
            .support
            .perform_pre_authentication_checks(user.as_ref())
            .await;

        if let Err(error) = check_result {
            if self.support.always_perform_additional_checks_on_user() {
                let _ = self
                    .additional_authentication_checks(user.clone(), authentication)
                    .await;
            }
            if !cache_was_used {
                return Err(error);
            }
            cache_was_used = false;
            user = self.retrieve_user(&username, authentication).await?;
            self.support
                .perform_pre_authentication_checks(user.as_ref())
                .await?;
            self.additional_authentication_checks(user.clone(), authentication)
                .await?;
        } else {
            self.additional_authentication_checks(user.clone(), authentication)
                .await?;
        }

        self.support
            .perform_post_authentication_checks(user.as_ref())
            .await?;

        if !cache_was_used {
            self.support
                .user_cache()
                .put_user_in_cache(username.clone(), user.clone());
        }

        let presented_password = authentication.get_credentials();
        self.check_compromised_password(presented_password.as_deref())?;
        let user = self
            .maybe_upgrade_password(user, presented_password.clone())
            .await;

        self.support
            .create_success_authentication(user.get_username().await, authentication, user.as_ref())
            .await
    }

    fn supports(&self, authentication: &str) -> bool {
        authentication == std::any::type_name::<UsernamePasswordAuthenticationToken>()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use next_web_core::{async_trait, error::BoxError};

    use crate::{
        authentication::{
            authentication_provider::AuthenticationProvider,
            password::{
                compromised_password_checker::CompromisedPasswordChecker,
                compromised_password_decision::CompromisedPasswordDecision,
            },
        },
        core::{
            authority_utils::AuthorityUtils,
            user_cache::InMemoryUserCache,
            userdetails::{
                user::User,
                user_details::UserDetails,
                user_details_password_service::UserDetailsPasswordService,
                user_details_service::UserDetailsService,
                username_not_found_error::UsernameNotFoundError,
            },
            username_password_authentication_token::UsernamePasswordAuthenticationToken,
        },
        crypto::password::password_encoder::PasswordEncoder,
    };

    use super::DaoAuthenticationProvider;

    struct PlaintextPasswordEncoder;

    impl PasswordEncoder for PlaintextPasswordEncoder {
        fn encode(&self, raw_password: &str) -> Result<String, BoxError> {
            Ok(format!("ENC:{raw_password}"))
        }

        fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
            encoded_password == format!("ENC:{raw_password}")
        }

        fn upgrade_encoding(&self, encoded_password: &str) -> bool {
            !encoded_password.starts_with("ENC:")
        }
    }

    struct LegacyUpgradingPasswordEncoder;

    impl PasswordEncoder for LegacyUpgradingPasswordEncoder {
        fn encode(&self, raw_password: &str) -> Result<String, BoxError> {
            Ok(format!("ENC:{raw_password}"))
        }

        fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
            encoded_password == raw_password || encoded_password == format!("ENC:{raw_password}")
        }

        fn upgrade_encoding(&self, encoded_password: &str) -> bool {
            !encoded_password.starts_with("ENC:")
        }
    }

    struct StubUserDetailsService {
        load_count: AtomicUsize,
        user: Arc<User>,
    }

    #[async_trait]
    impl UserDetailsService for StubUserDetailsService {
        async fn load_user_by_username(
            &self,
            username: String,
        ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
            self.load_count.fetch_add(1, Ordering::SeqCst);
            if username == self.user.username() {
                Ok(self.user.clone())
            } else {
                Err(UsernameNotFoundError(username))
            }
        }
    }

    struct RecordingPasswordService {
        update_count: AtomicUsize,
    }

    #[async_trait]
    impl UserDetailsPasswordService for RecordingPasswordService {
        async fn update_password(
            &self,
            user: Arc<dyn UserDetails>,
            _new_password: Option<String>,
        ) -> Arc<dyn UserDetails> {
            self.update_count.fetch_add(1, Ordering::SeqCst);
            user
        }
    }

    struct AlwaysCompromisedChecker;

    impl CompromisedPasswordChecker for AlwaysCompromisedChecker {
        fn check(&self, _password: Option<&str>) -> CompromisedPasswordDecision {
            CompromisedPasswordDecision::new(true)
        }
    }

    #[tokio::test]
    async fn dao_authentication_provider_authenticates_username_password_token() {
        let service = Arc::new(StubUserDetailsService {
            load_count: AtomicUsize::new(0),
            user: Arc::new(User::new(
                "alice",
                Some(String::from("ENC:secret")),
                AuthorityUtils::create_authority_list(["ROLE_USER"]),
            )),
        });
        let mut provider = DaoAuthenticationProvider::new(service);
        provider.set_password_encoder(Arc::new(PlaintextPasswordEncoder));

        let input = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        let authentication = provider.authenticate(&input).await.unwrap();

        assert!(authentication.is_authenticated());
        assert_eq!(authentication.get_name(), "alice");
        assert_eq!(authentication.authorities(), vec![String::from("ROLE_USER")]);
    }

    #[tokio::test]
    async fn dao_authentication_provider_reuses_cached_user() {
        let service = Arc::new(StubUserDetailsService {
            load_count: AtomicUsize::new(0),
            user: Arc::new(User::new(
                "alice",
                Some(String::from("ENC:secret")),
                AuthorityUtils::create_authority_list(["ROLE_USER"]),
            )),
        });
        let mut provider = DaoAuthenticationProvider::new(service.clone());
        provider.set_password_encoder(Arc::new(PlaintextPasswordEncoder));
        provider
            .support_mut()
            .set_user_cache(Arc::new(InMemoryUserCache::default()));

        let input = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        provider.authenticate(&input).await.unwrap();
        provider.authenticate(&input).await.unwrap();

        assert_eq!(service.load_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn dao_authentication_provider_upgrades_password_when_encoder_requests_it() {
        let service = Arc::new(StubUserDetailsService {
            load_count: AtomicUsize::new(0),
            user: Arc::new(User::new(
                "alice",
                Some(String::from("secret")),
                AuthorityUtils::create_authority_list(["ROLE_USER"]),
            )),
        });
        let recorder = Arc::new(RecordingPasswordService {
            update_count: AtomicUsize::new(0),
        });
        let mut provider = DaoAuthenticationProvider::new(service);
        provider.set_password_encoder(Arc::new(LegacyUpgradingPasswordEncoder));
        provider.set_user_details_password_service(recorder.clone());

        let input = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        let result = provider.authenticate(&input).await.unwrap();

        assert!(result.is_authenticated());
        assert_eq!(recorder.update_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn dao_authentication_provider_rejects_compromised_passwords() {
        let service = Arc::new(StubUserDetailsService {
            load_count: AtomicUsize::new(0),
            user: Arc::new(User::new(
                "alice",
                Some(String::from("ENC:secret")),
                AuthorityUtils::create_authority_list(["ROLE_USER"]),
            )),
        });
        let mut provider = DaoAuthenticationProvider::new(service);
        provider.set_password_encoder(Arc::new(PlaintextPasswordEncoder));
        provider.set_compromised_password_checker(Arc::new(AlwaysCompromisedChecker));

        let input = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        let error = match provider.authenticate(&input).await {
            Ok(_) => panic!("expected compromised password authentication to fail"),
            Err(error) => error,
        };

        assert_eq!(
            error.get_message(),
            "The provided password is compromised, please change your password"
        );
    }
}
