use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use next_web_core::http::StatusCode;
use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::debug;

use crate::{
    core::{
        authentication_error::{AuthenticationError, AuthenticationErrorKind},
        context::{security_context_holder::SecurityContextHolder, SecurityContextHolderStrategy},
        user_cache::{NullUserCache, UserCache},
        userdetails::{UserDetails, UserDetailsService},
        username_password_authentication_token::UsernamePasswordAuthenticationToken,
    },
    web::{
        authentication::www::{
            digest_data::DigestData, nonce_expired_exception::NonceExpiredException,
        },
        context::SecurityContextRepository,
    },
};

use super::digest_auth_utils;

/// Processes a HTTP request's Digest authorization headers, placing the result
/// into the `SecurityContextHolder`.
///
/// For a detailed background on what this filter is designed to process, refer to
/// [RFC 2617](https://www.ietf.org/rfc/rfc2617.txt) (which superseded RFC 2069,
/// although this filter supports clients that implement either RFC 2617 or RFC 2069).
///
/// This Digest implementation has been designed to avoid needing to store session
/// state between invocations. All session management information is stored in the
/// "nonce" that is sent to the client.
///
/// If authentication is successful, the resulting `Authentication` object will be
/// placed into the `SecurityContextHolder`.
#[derive(Clone)]
pub struct DigestAuthenticationFilter {
    /// The security context holder strategy.
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    /// The secret key used to generate and validate nonces.
    key: String,

    /// The realm name presented to the client in the WWW-Authenticate challenge.
    realm_name: String,

    /// The number of seconds that a nonce is valid for. Default: 300 seconds.
    nonce_validity_seconds: u64,

    /// If `true`, the password stored in `UserDetails` is already in A1 MD5 format
    /// (i.e. `MD5(username:realm:password)`). If `false`, the password is plain text.
    password_already_encoded: bool,

    /// If `true`, the `Authentication` object created after successful digest
    /// authentication will be marked as authenticated and filled with the
    /// authorities loaded by the `UserDetailsService`. Default: `false`.
    create_authenticated_token: bool,

    /// The `UserDetailsService` used to load user information.
    user_details_service: Arc<dyn UserDetailsService>,

    /// The `UserCache` used to cache `UserDetails`. Defaults to `NullUserCache`.
    user_cache: Arc<dyn UserCache>,

    /// The security context repository used to persist the security context
    /// on authentication success. Defaults to `None`.
    security_context_repository: Option<Arc<dyn SecurityContextRepository>>,
}

impl DigestAuthenticationFilter {
    /// Creates a new `DigestAuthenticationFilter`.
    ///
    /// # Parameters
    /// * `user_details_service` - The service used to load user information.
    /// * `key`                  - The secret key for nonce generation/validation.
    /// * `realm_name`           - The realm name for the Digest challenge.
    pub fn new(
        user_details_service: Arc<dyn UserDetailsService>,
        key: impl Into<String>,
        realm_name: impl Into<String>,
    ) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            key: key.into(),
            realm_name: realm_name.into(),
            nonce_validity_seconds: 300,
            password_already_encoded: false,
            create_authenticated_token: false,
            user_details_service,
            user_cache: Arc::new(NullUserCache),
            security_context_repository: None,
        }
    }

    // --- Getters / Setters ---

    /// Returns the secret key used for nonce generation and validation.
    pub fn get_key(&self) -> &str {
        &self.key
    }

    /// Sets the secret key used for nonce generation and validation.
    pub fn set_key(&mut self, key: impl Into<String>) {
        self.key = key.into();
    }

    /// Returns the realm name.
    pub fn get_realm_name(&self) -> &str {
        &self.realm_name
    }

    /// Sets the realm name.
    pub fn set_realm_name(&mut self, realm_name: impl Into<String>) {
        self.realm_name = realm_name.into();
    }

    /// Returns the nonce validity in seconds.
    pub fn get_nonce_validity_seconds(&self) -> u64 {
        self.nonce_validity_seconds
    }

    /// Sets the number of seconds a nonce is valid for.
    pub fn set_nonce_validity_seconds(&mut self, seconds: u64) {
        self.nonce_validity_seconds = seconds;
    }

    /// Returns whether the stored password is already in A1 format.
    pub fn is_password_already_encoded(&self) -> bool {
        self.password_already_encoded
    }

    /// If `true`, the password stored in `UserDetails` is assumed to already be
    /// in A1 format (`MD5(username:realm:password)`).
    pub fn set_password_already_encoded(&mut self, encoded: bool) {
        self.password_already_encoded = encoded;
    }

    /// Returns whether to create an authenticated token directly.
    pub fn is_create_authenticated_token(&self) -> bool {
        self.create_authenticated_token
    }

    /// If set to `true`, the `Authentication` object created after successful
    /// digest authentication will be marked as authenticated and filled with
    /// the authorities loaded by the `UserDetailsService`. This means only
    /// the password is checked, but not flags like `isEnabled()` or
    /// `isAccountNonExpired()`.
    pub fn set_create_authenticated_token(&mut self, create: bool) {
        self.create_authenticated_token = create;
    }

    /// Returns the `UserDetailsService`.
    pub fn get_user_details_service(&self) -> &dyn UserDetailsService {
        self.user_details_service.as_ref()
    }

    /// Sets the `UserDetailsService`.
    pub fn set_user_details_service(&mut self, service: Arc<dyn UserDetailsService>) {
        self.user_details_service = service;
    }

    /// Returns the user cache.
    pub fn get_user_cache(&self) -> &dyn UserCache {
        self.user_cache.as_ref()
    }

    /// Sets the `UserCache`.
    pub fn set_user_cache(&mut self, cache: Arc<dyn UserCache>) {
        self.user_cache = cache;
    }

    /// Sets the `SecurityContextRepository` to save the `SecurityContext` on
    /// authentication success.
    pub fn set_security_context_repository(&mut self, repo: Arc<dyn SecurityContextRepository>) {
        self.security_context_repository = Some(repo);
    }

    /// Sets the `SecurityContextHolderStrategy`.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }

    // --- Internal methods ---

    /// Returns the current time in milliseconds since UNIX epoch.
    fn current_time_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    /// Generates a nonce value. Format: `base64(expirationTime + ":" + md5Hex(expirationTime + ":" + key))`.
    fn generate_nonce_value(&self) -> String {
        let expiry_time = Self::current_time_millis() + (self.nonce_validity_seconds as i64 * 1000);
        let signature_value = digest_auth_utils::md5_hex(&format!("{}:{}", expiry_time, self.key));
        let nonce_value = format!("{}:{}", expiry_time, signature_value);

        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(nonce_value.as_bytes())
    }

    /// Handles authentication failure: clears the security context, sets the
    /// WWW-Authenticate challenge header, and sends a 401 response.
    fn fail(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: &AuthenticationError,
    ) -> Result<(), FilterError> {
        let context = self.security_context_holder_strategy.create_empty_context();
        self.security_context_holder_strategy.set_context(context);
        debug!("Digest authentication failed: {:?}", error);

        // Build the WWW-Authenticate header.
        let nonce_value_base64 = self.generate_nonce_value();
        let mut authenticate_header = format!(
            "Digest realm=\"{}\", qop=\"auth\", nonce=\"{}\"",
            self.realm_name, nonce_value_base64
        );

        // Check if the error is a NonceExpiredException to signal stale=true.
        if error.kind() == AuthenticationErrorKind::CredentialsNotFound {
            // In the Java code, this is detected via `instanceof NonceExpiredException`.
            // We use a simplified check here.
        }
        // Append stale directive if nonce expired.
        if error.get_message().contains("expired") || error.get_message().contains("Nonce") {
            authenticate_header.push_str(", stale=\"true\"");
        }

        debug!(
            "WWW-Authenticate header sent to user agent: {}",
            authenticate_header
        );
        response.append_header("WWW-Authenticate", &authenticate_header);
        response.set_status_code(StatusCode::UNAUTHORIZED);

        Ok(())
    }

    /// Creates a successful `Authentication` token from the user details.
    fn create_successful_authentication(
        user: &dyn UserDetails,
        create_authenticated: bool,
    ) -> UsernamePasswordAuthenticationToken {
        if create_authenticated {
            // Build an authenticated token with authorities.
            // Note: get_authorities() and get_password() are async on UserDetails,
            // so we use block_on here since this is a sync helper.
            let username = user.username();
            let password = user.password();
            let authorities = user.authorities();
            let mut auth_list: Vec<Arc<dyn crate::core::granted_authority::GrantedAuthority>> =
                Vec::new();
            for auth in authorities {
                let name = auth.authority().unwrap_or_default();
                auth_list.push(Arc::new(
                    crate::core::simple_granted_authority::SimpleGrantedAuthority::new(name),
                ));
            }
            UsernamePasswordAuthenticationToken::authenticated(
                username,
                password.map(ToString::to_string),
                auth_list,
            )
        } else {
            UsernamePasswordAuthenticationToken::unauthenticated(
                user.username().to_string(),
                user.password().map(ToString::to_string),
            )
        }
    }
}

#[async_trait]
impl HttpFilter for DigestAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // Check for Digest Authorization header.
        let header = request.header("Authorization");
        let header = match header {
            Some(h) if h.starts_with("Digest ") => h,
            _ => {
                return filter_chain.do_filter(request, response).await;
            }
        };

        debug!(
            "Digest Authorization header received from user agent: {}",
            header
        );

        let mut digest_auth = DigestData::new(header);

        // Validate the digest data and decode the nonce.
        if let Err(ex) = digest_auth.validate_and_decode(&self.key, &self.realm_name) {
            return self.fail(request, response, &ex);
        }

        // Lookup password for presented username.
        let username = digest_auth
            .get_username()
            .expect("username validated in validate_and_decode");
        let mut cache_was_used = true;
        let mut user = self.user_cache.get_user_from_cache(username);

        // Load user from DAO if not cached.
        if user.is_none() {
            cache_was_used = false;
            user = Some(
                self.user_details_service
                    .load_user_by_username(username.to_string())
                    .await
                    .map_err(|_| {
                        let msg = format!("Username {} not found", username);
                        FilterError::custom(msg)
                    })?,
            );
            if let Some(ref u) = user {
                self.user_cache
                    .put_user_in_cache(username.to_string(), u.clone());
            }
        }

        let mut user = user.expect("user should be loaded");

        // Calculate the expected server digest.
        let http_method = request.method().to_string();
        let password = user.password();
        let mut server_digest_md5 = digest_auth.calculate_server_digest(
            password,
            &http_method,
            self.password_already_encoded,
        );

        // If the digest doesn't match and the user was cached, try refreshing from DAO.
        let client_response = digest_auth.get_response().unwrap_or("");
        if server_digest_md5 != client_response && cache_was_used {
            debug!(
                "Digest comparison failure; trying to refresh user from DAO in case password had changed"
            );
            user = self
                .user_details_service
                .load_user_by_username(username.to_string())
                .await
                .map_err(|_| {
                    let msg = format!("Username {} not found", username);
                    FilterError::custom(msg)
                })?;
            self.user_cache
                .put_user_in_cache(username.to_string(), user.clone());
            let refreshed_password = user.password();
            server_digest_md5 = digest_auth.calculate_server_digest(
                refreshed_password,
                &http_method,
                self.password_already_encoded,
            );
        }

        // If digest is still incorrect, reject.
        if server_digest_md5 != client_response {
            debug!(
                "Expected response: '{}' but received: '{}'; is UserDetailsService returning clear text passwords?",
                server_digest_md5, client_response
            );
            let error = AuthenticationError::with_kind(
                "Incorrect response",
                AuthenticationErrorKind::BadCredentials,
            );
            return self.fail(request, response, &error);
        }

        // Check if the nonce has expired. We do this last so we can direct
        // the user agent that its nonce is stale.
        if digest_auth.is_nonce_expired(Self::current_time_millis()) {
            let ex = NonceExpiredException::new("Nonce has expired/timed out");
            let auth_error = AuthenticationError::from(ex);
            return self.fail(request, response, &auth_error);
        }

        debug!(
            "Authentication success for user: '{}' with response: '{}'",
            username, client_response
        );

        // Create successful authentication and place in the security context.
        let authentication =
            Self::create_successful_authentication(user.as_ref(), self.create_authenticated_token);
        let context = self.security_context_holder_strategy.create_empty_context();
        context.set_authentication(Some(Arc::new(authentication)));
        self.security_context_holder_strategy
            .set_context(context.clone());
        if let Some(repo) = &self.security_context_repository {
            repo.save_context(&context, request, response).await;
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for DigestAuthenticationFilter {
    fn name(&self) -> &str {
        "DigestAuthenticationFilter"
    }
}
