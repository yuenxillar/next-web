use std::fmt::Debug;

use crate::any::AnyValue;

/// Provides a way to identify a user across more than one page request or visit
/// to a website and to store information about that user.
///
/// The servlet container uses this interface to create a session between an
/// HTTP client and an HTTP server. The session persists for a specified time
/// period, across more than one connection or page request from the user. A
/// session usually corresponds to one user, who may visit a site many times.
/// The server can maintain a session in many ways such as using cookies or
/// rewriting URLs.
pub trait HttpSession
where
    Self: Send + Sync,
    Self: Debug,
{
    /// Returns the time when this session was created, measured in milliseconds
    /// since midnight January 1, 1970 GMT.
    ///
    /// # Returns
    ///
    /// A `long` specifying when this session was created, expressed in
    /// milliseconds since 1/1/1970 GMT.
    fn creation_time(&self) -> u64;

    /// Returns a string containing the unique identifier assigned to this
    /// session. The identifier is assigned by the servlet container and is
    /// implementation dependent.
    ///
    /// # Returns
    ///
    /// A string specifying the identifier assigned to this session.
    fn id(&self) -> &str;

    /// Returns the last time the client sent a request associated with this
    /// session, as the number of milliseconds since midnight January 1, 1970
    /// GMT, and marked by the time the container received the request.
    ///
    /// Actions that your application takes, such as getting or setting a value
    /// associated with the session, do not affect the access time.
    ///
    /// # Returns
    ///
    /// A `long` representing the last time the client sent a request associated
    /// with this session, expressed in milliseconds since 1/1/1970 GMT.
    fn last_accessed_time(&self) -> u64;

    /// Specifies the time, in seconds, between client requests before the
    /// servlet container will invalidate this session. A zero or negative time
    /// indicates that the session should never time out.
    ///
    /// # Parameters
    ///
    /// - `interval`: An integer specifying the number of seconds.
    fn set_max_inactive_interval(&mut self, interval: u64);

    /// Returns the maximum time interval, in seconds, that the servlet container
    /// will keep this session open between client accesses. After this interval,
    /// the servlet container will invalidate the session. The maximum time
    /// interval can be set with the [`set_max_inactive_interval`] method. A zero
    /// or negative time indicates that the session should never time out.
    ///
    /// # Returns
    ///
    /// An integer specifying the number of seconds this session remains open
    /// between client requests.
    ///
    /// # See also
    ///
    /// - [`set_max_inactive_interval`]
    fn max_inactive_interval(&self) -> u64;

    /// Returns the object bound with the specified name in this session, or
    /// `null` if no object is bound under the name.
    ///
    /// # Parameters
    ///
    /// - `name`: A string specifying the name of the object.
    ///
    /// # Returns
    ///
    /// The object with the specified name.
    fn attribute(&self, name: &str) -> Option<&AnyValue>;

    /// Returns an [`Enumeration`] of `String` objects containing the names of
    /// all the objects bound to this session.
    ///
    /// # Returns
    ///
    /// objects bound to this session.
    fn attribute_names(&self) -> Vec<&str>;

    /// Binds an object to this session, using the name specified. If an object
    /// of the same name is already bound to the session, the object is replaced.
    ///
    /// After this method executes, and if the new object implements
    /// [`HttpSessionBindingListener`], the container calls
    /// [`HttpSessionBindingListener::value_bound`]. The container then notifies
    /// any [`HttpSessionAttributeListener`]s in the web application.
    ///
    /// If an object was already bound to this session of this name that
    /// implements [`HttpSessionBindingListener`], its
    /// [`HttpSessionBindingListener::value_unbound`] method is called.
    ///
    /// If the value passed in is null, this has the same effect as calling
    /// [`remove_attribute`].
    ///
    /// # Parameters
    ///
    /// - `name`: The name to which the object is bound; cannot be null.
    /// - `value`: The object to be bound.
    fn set_attribute(&mut self, name: &str, value: AnyValue);

    /// Removes the object bound with the specified name from this session. If
    /// the session does not have an object bound with the specified name, this
    /// method does nothing.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the object to remove from this session.
    fn remove_attribute(&mut self, name: &str);

    /// Invalidates this session then unbinds any objects bound to it.
    fn invalidate(&mut self);

    /// Returns `true` if the client does not yet know about the session or if
    /// the client chooses not to join the session. For example, if the server
    /// used only cookie-based sessions, and the client had disabled the use of
    /// cookies, then a session would be new on each request.
    ///
    /// # Returns
    ///
    /// `true` if the server has created a session, but the client has not yet
    /// joined.
    fn is_new(&self) -> bool;

    /// Provides a mechanism for applications to interact with the
    /// [`HttpSession`] outside of the scope of an HTTP request.
    ///
    /// # Returns
    ///
    /// An [`HttpSessionAccessor`] instance linked to the current session ID (if the session
    /// ID is changed the [`HttpSessionAccessor`] will no longer be able to access this
    /// session).
    fn accessor(&self) -> Option<Box<dyn HttpSessionAccessor>> {
        None
    }
}

/// Provides a mechanism for applications to interact with the [`HttpSession`]
/// outside of the scope of an HTTP request.
pub trait HttpSessionAccessor {
    /// Call to access the session with the same semantics as if the session was
    /// accessed during an HTTP request.
    ///
    /// The effect of this call on the session is as if an HTTP request starts;
    /// [`Consumer::accept`] is called with the associated session object
    /// enabling the application to interact with the session; and, once that
    /// method returns, the HTTP request ends.
    ///
    /// # Parameters
    ///
    /// - `session_consumer`: The application provided [`Consumer`] instance that
    ///   will access the session.
    ///
    /// # Errors
    ///
    /// Returns [`IllegalStateException`] if the session with the ID to which the
    /// [`Accessor`] is associated is no longer valid.
    fn access(&self, session_consumer: Box<dyn FnOnce(&dyn HttpSession)>);
}
