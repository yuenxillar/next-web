use std::any::Any;
use std::fmt;
use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::core::granted_authority::GrantedAuthority;
use crate::core::userdetails::user::User;
use crate::core::userdetails::UserDetails;
use crate::core::username_password_authentication_token::UsernamePasswordAuthenticationToken;
use crate::core::{AuthenticatedPrincipal, Principal};
use crate::core::{Authentication, CredentialsContainer};
use crate::web::authentication::AuthPrincipal;

#[derive(Clone)]
pub struct BaseAuthenticationToken {
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    details: Option<AuthPrincipal>,
    authenticated: bool,
}

impl BaseAuthenticationToken {
    /// Creates a token with the supplied array of authorities.
    ///
    /// # Arguments
    ///
    /// * `authorities` - the collection of `GrantedAuthority`s for the principal
    ///   represented by this authentication object.
    pub fn new(authorities: Option<Vec<Arc<dyn GrantedAuthority>>>) -> Self {
        let authorities = match authorities {
            Some(auths) => auths,
            None => Vec::new(), // NO_AUTHORITIES equivalent
        };

        Self {
            authorities,
            details: None,
            authenticated: false,
        }
    }

    /// Creates a token from a builder.
    ///
    /// # Arguments
    ///
    /// * `builder` - the builder containing the token configuration
    pub fn from_builder(mut builder: BaseAuthenticationBuilder) -> Self {
        Self {
            authorities: std::mem::take(&mut builder.authorities),
            details: builder.details.take(),
            authenticated: builder.authenticated,
        }
    }

    pub fn set_details(&mut self, details: Option<AuthPrincipal>) {
        self.details = details;
    }

    fn erase_secret(&self, secret: Option<&AuthPrincipal>) {
        if let Some(secret) = secret {
            if let Some(user) = secret.downcast_ref::<User>() {
                user.erase_credentials();
            }

            if let Some(user) = secret.downcast_ref::<UsernamePasswordAuthenticationToken>() {
                user.erase_credentials();
            }
        }
    }
}

impl Authentication for BaseAuthenticationToken {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        None
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        None
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn set_authenticated(&mut self, authenticated: bool) -> Result<(), BoxError> {
        self.authenticated = authenticated;

        Ok(())
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.details.as_ref()
    }
}

impl Principal for BaseAuthenticationToken {
    /// Returns the name of the principal.
    ///
    /// Checks for `UserDetails`, `AuthenticatedPrincipal`, or `Principal` trait
    /// implementations to determine the name. Falls back to the string representation
    /// of the principal.
    fn name(&self) -> &str {
        let principal = self.principal();

        // Check for UserDetails
        if let Some(user_details) = principal.and_then(|p| p.downcast_ref::<Arc<dyn UserDetails>>())
        {
            return user_details.username();
        }

        // Check for AuthenticatedPrincipal
        if let Some(auth_principal) =
            principal.and_then(|p| p.downcast_ref::<Arc<dyn AuthenticatedPrincipal>>())
        {
            return auth_principal.name();
        }

        // Check for standard Principal
        if let Some(standard_principal) =
            principal.and_then(|p| p.downcast_ref::<Arc<dyn Principal>>())
        {
            return standard_principal.name();
        }

        principal
            .and_then(|s| s.downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or_default()
    }
}

impl CredentialsContainer for BaseAuthenticationToken {
    fn erase_credentials(&self) {
        self.erase_secret(self.credentials());
        self.erase_secret(self.principal());
        self.erase_secret(self.details.as_ref());
    }
}

impl PartialEq for BaseAuthenticationToken {
    fn eq(&self, other: &Self) -> bool {
        // Compare authorities
        if self.authorities.len() != other.authorities.len() {
            return false;
        }

        for (a, b) in self.authorities.iter().zip(other.authorities.iter()) {
            if !Arc::ptr_eq(a, b) && a.authority() != b.authority() {
                return false;
            }
        }

        // Compare details
        match (&self.details, &other.details) {
            (None, Some(_)) | (Some(_), None) => return false,
            (Some(a), Some(b)) => {
                // For trait objects, we compare type IDs and debug representations
                if a.as_ref().type_id() != b.as_ref().type_id() {
                    return false;
                }
            }
            (None, None) => {}
        }

        // Compare credentials
        let self_creds = self.credentials();
        let other_creds = other.credentials();
        match (self_creds, other_creds) {
            (None, Some(_)) | (Some(_), None) => return false,
            (Some(a), Some(b)) => {
                if a.type_id() != b.type_id() {
                    return false;
                }
            }
            (None, None) => {}
        }

        // Compare principal
        let self_principal = self.principal();
        let other_principal = other.principal();
        match (&self_principal, &other_principal) {
            (None, Some(_)) | (Some(_), None) => return false,
            (Some(a), Some(b)) => {
                if a.as_ref().type_id() != b.as_ref().type_id() {
                    return false;
                }
            }
            (None, None) => {}
        }

        // Compare authenticated state
        self.is_authenticated() == other.is_authenticated()
    }
}

impl Eq for BaseAuthenticationToken {}

impl fmt::Display for BaseAuthenticationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [Principal={:?}, Credentials=[PROTECTED], Authenticated={}, Details={:?}, Granted Authorities={:?}]",
            std::any::type_name::<Self>(),
            self.principal(),
            self.is_authenticated(),
            self.details,
            self.authorities.iter().map(|a| a.authority()).collect::<Vec<_>>()
        )
    }
}

/// A common abstract implementation of `Authentication.Builder`. It implements
/// the builder methods that correspond to the `Authentication` methods that
/// `BaseAuthenticationToken` implements.
pub struct BaseAuthenticationBuilder {
    authenticated: bool,
    details: Option<AuthPrincipal>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl BaseAuthenticationBuilder {
    /// Creates a new builder from an existing token.
    ///
    /// # Arguments
    ///
    /// * `token` - the token to initialize the builder from
    pub fn new(token: &BaseAuthenticationToken) -> Self {
        Self {
            authorities: token.authorities.clone(),
            authenticated: token.is_authenticated(),
            details: token.details.clone(),
        }
    }

    /// Sets the authenticated state.
    ///
    /// # Arguments
    ///
    /// * `authenticated` - the authenticated state
    ///
    /// # Returns
    ///
    /// The builder for method chaining
    pub fn authenticated(mut self, authenticated: bool) -> Self {
        self.authenticated = authenticated;
        self
    }

    /// Sets the details.
    ///
    /// # Arguments
    ///
    /// * `details` - the details to set
    ///
    /// # Returns
    ///
    /// The builder for method chaining
    pub fn details(mut self, details: Option<AuthPrincipal>) -> Self {
        self.details = details;
        self
    }

    /// Modifies the authorities using a consumer function.
    ///
    /// # Arguments
    ///
    /// * `authorities_consumer` - function that modifies the authorities collection
    ///
    /// # Returns
    ///
    /// The builder for method chaining
    pub fn authorities<F>(mut self, authorities: F) -> Self
    where
        F: FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>),
    {
        authorities(&mut self.authorities);
        self.authenticated = true;
        self
    }
}
