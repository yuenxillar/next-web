use std::sync::Arc;

use matchit::Router;

use crate::ws_handler::WebSocketHandler;

/// A registry that maps URL path patterns to WebSocket handlers.
///
/// This struct maintains a routing table for WebSocket connections, allowing
/// different handlers to be invoked based on the request path. It uses a
/// [`matchit::Router`] internally for efficient pattern matching with support
/// for path parameters and wildcards.
///
/// # Features
///
/// - Pattern-based routing (e.g., `/ws/chat/{room_id}`)
/// - Thread-safe handler storage using [`Arc`]
/// - Maintains original path strings for introspection
///
/// # Examples
///
/// ```rust
/// use std::sync::Arc;
/// use matchit::Router;
///
/// let mut mapping = WebSocketHandlerMapping::default();
///
/// // Register a handler for a specific path pattern
/// mapping.insert("/ws/chat/{room_id}", Arc::new(MyChatHandler));
///
/// // Look up a handler for an incoming request
/// if let Some(handler) = mapping.get("/ws/chat/room123") {
///     // Handle the WebSocket connection
/// }
/// ```
#[derive(Clone)]
pub struct WebSocketHandlerMapping {
    /// Stores the original path strings in the order they were registered.
    /// Useful for debugging, introspection, or listing all available endpoints.
    original_paths: Vec<String>,

    /// The underlying router that performs pattern matching and stores handler references.
    /// Each handler is wrapped in an [`Arc`] to allow shared ownership across threads.
    handlers: Router<Arc<dyn WebSocketHandler>>,
}

impl WebSocketHandlerMapping {
    /// Creates a new mapping with a pre-configured router.
    ///
    /// This constructor is primarily used when you need to initialize the mapping
    /// with an existing router that may already contain handlers.
    ///
    /// # Arguments
    ///
    /// * `handlers` - A [`matchit::Router`] containing the initial set of handlers
    ///
    /// # Returns
    ///
    /// A new `WebSocketHandlerMapping` instance with an empty `original_paths` list.
    ///
    /// # Note
    ///
    /// The `original_paths` field is initialized as empty because the router's
    /// existing routes are not tracked. Use [`insert`] to add routes while
    /// maintaining the path history.
    ///
    /// [`insert`]: WebSocketHandlerMapping::insert
    pub fn new(handlers: Router<Arc<dyn WebSocketHandler>>) -> Self {
        Self {
            handlers,
            original_paths: Default::default(),
        }
    }

    /// Registers a new WebSocket handler for the specified path pattern.
    ///
    /// This method adds a route to the internal router and records the original
    /// path string for later introspection. The handler is stored in an [`Arc`]
    /// to enable safe sharing across multiple connections.
    ///
    /// # Arguments
    ///
    /// * `path` - A URL path pattern that may contain parameters (e.g., `/ws/{id}`)
    /// * `handler` - The WebSocket handler wrapped in an [`Arc`]
    ///
    /// # Panics
    ///
    /// Panics if the route cannot be inserted into the router. This typically
    /// happens when:
    /// - The path pattern contains invalid syntax
    /// - A conflicting route already exists
    /// - The router implementation encounters an internal error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::Arc;
    ///
    /// let mut mapping = WebSocketHandlerMapping::default();
    ///
    /// // Register a simple handler
    /// mapping.insert("/ws/echo", Arc::new(EchoHandler));
    ///
    /// // Register a parameterized handler
    /// mapping.insert("/ws/room/{room_id}", Arc::new(RoomHandler));
    /// ```
    pub fn insert(&mut self, path: &str, handler: Arc<dyn WebSocketHandler>) {
        self.handlers
            .insert(path, handler)
            .and_then(|_| {
                self.original_paths.push(path.to_string());
                Ok(())
            })
            .unwrap_or_else(|e| panic!("Failed to add handler: {:?}", e));
    }

    /// Retrieves a WebSocket handler for the given request path.
    ///
    /// This method matches the provided path against all registered patterns
    /// and returns the corresponding handler if a match is found. The matching
    /// is performed using the rules defined by the [`matchit::Router`], which
    /// supports static paths, named parameters, and wildcards.
    ///
    /// # Arguments
    ///
    /// * `path` - The actual request path to match against registered patterns
    ///
    /// # Returns
    ///
    /// * `Some(&Arc<dyn WebSocketHandler>)` - A reference to the matched handler
    /// * `None` - If no registered pattern matches the provided path
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mapping = WebSocketHandlerMapping::default();
    /// // ... register handlers ...
    ///
    /// // Try to find a handler for an incoming request
    /// match mapping.get("/ws/chat/room42") {
    ///     Some(handler) => {
    ///         // Upgrade the connection and delegate to the handler
    ///     }
    ///     None => {
    ///         // Return 404 or reject the WebSocket upgrade
    ///     }
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This method does not provide access to captured path parameters.
    /// If you need access to parameters, consider using the underlying
    /// router directly or extending this API.
    pub fn get(&self, path: &str) -> Option<&Arc<dyn WebSocketHandler>> {
        match self.handlers.at(path) {
            Ok(matched) => Some(matched.value),
            Err(_) => None,
        }
    }

    /// Returns a slice of all registered path patterns.
    ///
    /// This method provides access to the original path strings in the order
    /// they were inserted. This is useful for debugging, generating API
    /// documentation, or listing available WebSocket endpoints.
    ///
    /// # Returns
    ///
    /// A slice containing references to all registered path patterns.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mapping = WebSocketHandlerMapping::default();
    /// // ... register handlers ...
    ///
    /// // Print all registered endpoints
    /// for path in mapping.paths() {
    ///     println!("WebSocket endpoint available at: {}", path);
    /// }
    /// ```
    pub fn paths(&self) -> &[String] {
        &self.original_paths
    }
}

impl Default for WebSocketHandlerMapping {
    /// Creates a new empty `WebSocketHandlerMapping`.
    ///
    /// This implementation provides a convenient way to create a mapping
    /// with no pre-registered handlers.
    ///
    /// # Returns
    ///
    /// A new instance with an empty router and no registered paths.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut mapping = WebSocketHandlerMapping::default();
    /// // Start adding handlers
    /// mapping.insert("/ws/endpoint", Arc::new(MyHandler));
    /// ```
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            original_paths: Default::default(),
        }
    }
}
