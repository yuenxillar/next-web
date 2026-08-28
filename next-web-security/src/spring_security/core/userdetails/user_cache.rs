use std::sync::Arc;

use crate::core::userdetails::UserDetails;

/// Provides a cache of [`UserDetails`] objects.
///
/// Implementations should provide appropriate methods to set their cache parameters (e.g.
/// time-to-live) and/or force removal of entities before their normal expiration. These
/// are not part of the `UserCache` trait contract because they vary depending on the
/// type of caching system used (in-memory, disk, cluster, hybrid etc.).
///
/// Caching is generally only required in applications which do not maintain server-side
/// state, such as remote clients or web services. The authentication credentials are then
/// presented on each invocation and the overhead of accessing a database or other
/// persistent storage mechanism to validate would be excessive. In this case, you would
/// configure a cache to store the `UserDetails` information rather than loading it
/// each time.
pub trait UserCache {
    /// Obtains a [`UserDetails`] from the cache.
    ///
    /// # Parameters
    /// * `username` - the username used to place the user in the cache
    ///
    /// # Returns
    /// The populated `UserDetails` or `None` if the user could not be found
    /// or if the cache entry has expired
    fn get_user_from_cache(&self, username: &str) -> Option<&Arc<dyn UserDetails>>;

    /// Places a [`UserDetails`] in the cache. The `username` is the key
    /// used to subsequently retrieve the `UserDetails`.
    ///
    /// # Parameters
    /// * `user` - the fully populated `UserDetails` to place in the cache
    fn put_user_in_cache(&self, user: Arc<dyn UserDetails>);

    /// Removes the specified user from the cache. The `username` is the key
    /// used to remove the user. If the user is not found, the method should simply return
    /// (not panic).
    ///
    /// Some cache implementations may not support eviction from the cache, in which case
    /// they should provide appropriate behaviour to alter the user in either its
    /// documentation, via a panic, or through a log message.
    ///
    /// # Parameters
    /// * `username` - the username to be evicted from the cache
    fn remove_user_from_cache(&self, username: &str);
}
