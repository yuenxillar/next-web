use std::error::Error;
use std::future::Future;

/// A trait for server types that can be run asynchronously.
///
/// Implementors of this trait represent a server that can be started and
/// driven to completion. The [`run`](Server::run) method is typically called
/// once during application startup and is expected to keep running until the
/// server shuts down, either gracefully or due to an error.
///
/// # Design Notes
///
/// The method returns `impl Future + 'a` instead of using `async fn` in the
/// trait, so that the returned future's lifetime is explicitly tied to the
/// borrow of `self`. This avoids the "hidden type captures lifetime" error
/// (E0700) that can occur when the compiler cannot infer the relationship
/// between the returned future and the input reference.
///
/// The `Box<dyn Error>` return type keeps the trait simple and usable across
/// different server implementations, each of which may produce its own error
/// type. Implementations can convert their concrete errors into
/// `Box<dyn Error>` via `?` or `.into()`.
///
/// # Examples
///
/// ```ignore
/// struct MyServer {
///     addr: SocketAddr,
/// }
///
/// impl Server for MyServer {
///     fn run<'a>(&'a mut self) -> impl Future<Output = Result<(), Box<dyn Error>>> + 'a {
///         async move {
///             let listener = tokio::net::TcpListener::bind(self.addr).await?;
///             // ... serve requests ...
///             Ok(())
///         }
///     }
/// }
/// ```
pub trait Server {
    /// Runs the server until it shuts down or fails.
    ///
    /// This method borrows `self` mutably for the entire duration of the
    /// returned future. The server is expected to keep running until either
    /// a shutdown signal is received or an unrecoverable error occurs.
    ///
    /// # Returns
    ///
    /// A future that resolves to `Ok(())` when the server shuts down
    /// gracefully, or an error if the server fails to start or encounters
    /// a fatal condition during operation.
    ///
    /// # Lifetimes
    ///
    /// The lifetime `'a` is tied to the mutable borrow of `self`, ensuring
    /// that the returned future cannot outlive the server instance. This
    /// makes the borrow relationship explicit and avoids lifetime inference
    /// issues in trait method implementations.
    fn run<'a>(&'a mut self) -> impl Future<Output = Result<(), Box<dyn Error>>> + 'a;
}
