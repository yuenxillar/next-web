/// Representation of an authenticated `Principal` once an
/// [`Authentication`] request has been successfully authenticated by the
/// [`AuthenticationManager::authenticate`] method.
///
/// Implementors typically provide their own representation of a `Principal`,
/// which usually contains information describing the `Principal` entity, such
/// as, first/middle/last name, address, email, phone, id, etc.
///
/// This trait allows implementors to expose specific attributes of their custom
/// representation of `Principal` in a generic way.
///
/// # See Also
/// - [`Authentication::principal`]
/// - [`UserDetails`]
pub trait AuthenticatedPrincipal
where
    Self: Send + Sync,
{
    /// Returns the name of the authenticated `Principal`.
    ///
    /// Never returns `None`.
    ///
    /// # Returns
    /// The name of the authenticated `Principal`.
    fn name(&self) -> &str;
}
