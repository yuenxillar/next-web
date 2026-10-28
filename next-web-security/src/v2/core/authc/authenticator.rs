use next_web_core::async_trait;

use super::{
    authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
    AuthenticationError,
};

/// The part of a security manager that turns a token into an authentication.
///
/// A token carries what the user presented — a user name and a password, a
/// bearer token, a host name — and the authenticator resolves it against the
/// realms of the application. The realm's lookup is asynchronous because a realm
/// may talk to a directory, so the authenticator is asynchronous as well.
#[async_trait]
pub trait Authenticator
where
    Self: Send + Sync,
{
    /// Returns the authentication of the given token.
    ///
    /// # Arguments
    ///
    /// * `authentication_token` - What the user presented.
    ///
    /// # Errors
    ///
    /// Returns the [`AuthenticationError`] of the first realm that refused the
    /// token, or the error of a configuration without a matching realm.
    async fn authenticate(
        &self,
        authentication_token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn AuthenticationInfo>, AuthenticationError>;
}
