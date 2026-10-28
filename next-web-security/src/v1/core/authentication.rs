use std::{
    any::{Any, TypeId},
    fmt::Debug,
    sync::Arc,
};

use next_web_core::error::BoxError;

use crate::{
    core::{GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};

/// Represents the token for an authentication request or for an authenticated
/// principal once the request has been processed by the
/// [`AuthenticationManager::authenticate`] method.
///
/// Once the request has been authenticated, the `Authentication` will usually
/// be stored in a thread-local `SecurityContext` managed by the
/// `SecurityContextHolder` by the authentication mechanism which is being used.
///
/// An explicit authentication can be achieved, without using one of the
/// authentication mechanisms, by creating an `Authentication` instance and
/// using the code:
///
/// ```rust,ignore
/// let mut context = SecurityContextHolder::create_empty_context();
/// context.set_authentication(an_authentication);
/// SecurityContextHolder::set_context(context);
/// ```
///
/// Note that unless the `Authentication` has the `authenticated` property set
/// to `true`, it will still be authenticated by any security interceptor
/// (for method or web invocations) which encounters it.
///
/// In most cases, the framework transparently takes care of managing the
/// security context and authentication objects for you.
pub trait Authentication
where
    Self: Send + Sync,
    Self: Any,
    Self: Principal,
    Self: Debug,
{
    /// Set by an AuthenticationManager to indicate the authorities that the principal has been granted.
    /// Note that classes should not rely on this value as being valid unless it has been set by a trusted AuthenticationManager.
    ///
    /// Implementations should ensure that modifications to the returned collection array do not affect
    /// the state of the Authentication object, or use an unmodifiable instance.
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>];

    /// The credentials that prove the principal is correct. This is usually a password, but could be anything
    /// relevant to the AuthenticationManager. Callers are expected to populate the credentials.
    fn credentials(&self) -> Option<&AuthPrincipal>;

    /// Stores additional details about the authentication request. These might be an IP address, certificate serial number etc.
    fn details(&self) -> Option<&AuthPrincipal>;

    /// The identity of the principal being authenticated. In the case of an authentication request with
    /// username and password, this would be the username. Callers are expected to populate the
    /// principal for an authentication request.
    ///
    /// The AuthenticationManager implementation will often return an Authentication containing
    /// richer information as the principal for use by the application. Many of the authentication providers
    /// will create a UserDetails object as the principal.
    fn principal(&self) -> Option<&AuthPrincipal>;

    /// Used to indicate to AbstractSecurityInterceptor whether it should present the authentication
    /// token to the AuthenticationManager. Typically an AuthenticationManager (or, more often,
    /// one of its AuthenticationProviders) will return an immutable authentication token after successful authentication,
    /// in which case that token can safely return true to this method. Returning true will improve performance, as calling the AuthenticationManager for every
    /// request will no longer be necessary.
    ///
    /// For security reasons, implementations of this interface should be very careful about returning
    /// true from this method unless they are either immutable, or have some way of ensuring the
    /// properties have not been changed since original creation.
    fn is_authenticated(&self) -> bool;

    /// See is_authenticated() for a full description.
    /// Implementations should always allow this method to be called with a false parameter, as this is
    /// used by various classes to specify the authentication token should not be trusted. If an
    /// implementation wishes to reject an invocation with a true parameter (which would indicate the
    /// The authentication token is trustworthy, which is a potential security risk, and the implementation should return an error
    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError>;

    /// Return an Authentication.Builder based on this instance. By default, returns a builder that builds a SimpleAuthentication.
    /// Although a default method, all Authentication implementations should implement this.
    /// The reason is to ensure that the Authentication type is preserved when Authentication.Builder.
    /// build is invoked. This is especially important in the event that your authentication implementation contains custom fields.
    ///
    /// This isn't strictly necessary since it is recommended that applications code to the Authentication interface and that
    /// custom information is often contained in the getPrincipal value.
    fn to_builder(&self) -> Box<dyn AuthenticationBuilder>;
    // Box::new(SimpleAuthenticationBuilder::new(self))

    /// Returns the TypeId of this Authentication instance.
    fn of(&self) -> TypeId;
}

/// A builder based on a given Authentication instance
pub trait AuthenticationBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>);

    fn credentials(&mut self, credentials: Option<AuthPrincipal>);

    fn details(&mut self, details: Option<AuthPrincipal>);

    fn principal(&mut self, principal: Option<AuthPrincipal>);

    fn authenticated(&mut self, authenticated: bool);

    fn build(&mut self) -> Arc<dyn Authentication>;
}
