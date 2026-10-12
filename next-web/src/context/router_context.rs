#[cfg(feature = "enable-open-api")]
use next_web_open_api::OpenApiRouter;
use next_web_singletons::factory::support::Key;
use std::sync::{Mutex, OnceLock};

/// Global singleton holding all registered dependency keys (`Key`).
static HANDLER_SINGLETON_DEPENDENCIES: OnceLock<Mutex<Vec<Key>>> = OnceLock::new();

/// Returns a mutable reference to the global dependency list.
fn dependencies() -> &'static Mutex<Vec<Key>> {
    HANDLER_SINGLETON_DEPENDENCIES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Shared state passed to every [`HttpHandlerAutoRegister::register`] call.
///
/// It currently only carries OpenAPI-related state, and the field is
/// conditionally compiled so that builds without the `enable-open-api`
/// feature do not pay for the dependency or the extra field.
#[derive(Debug, Clone)]
pub struct RouterContext {
    /// Keys of handler singletons that should be preloaded into the
    /// application context.
    ///
    /// Populated as handlers register themselves, and drained by
    /// [`preload_handler_singletons`] once the router setup is complete.
    singleton_dependencies: Vec<Key>,

    /// Accumulated OpenAPI specification, if the `enable-open-api`
    /// feature is enabled.
    ///
    /// It is `None` until [`RouterContext::with_openapi`] is called, and
    /// is later taken out by the router setup once all handlers have
    /// registered their paths.
    #[cfg(feature = "enable-open-api")]
    pub open_api: Option<OpenApiRouter>,
}

impl RouterContext {
    /// Returns an empty context.
    ///
    /// When the `enable-open-api` feature is enabled, `open_api` starts
    /// as `None`; callers that need OpenAPI support should use
    /// [`RouterContext::with_openapi`] instead.
    pub fn new() -> Self {
        Self {
            singleton_dependencies: Vec::new(),
            #[cfg(feature = "enable-open-api")]
            open_api: None,
        }
    }

    /// Registers a singleton dependency.
    pub fn register_singleton_dependency(&mut self, key: Key) {
        if !self.singleton_dependencies.contains(&key) {
            self.singleton_dependencies.push(key.clone());
        }
        let mut global = dependencies()
            .lock()
            .expect("handler dependency lock poisoned");
        if !global.contains(&key) {
            global.push(key);
        }
    }

    /// Returns the singleton dependencies for handlers.
    pub fn take_singleton_dependencies(&mut self) -> Vec<Key> {
        let mut global = dependencies()
            .lock()
            .expect("handler dependency lock poisoned");
        let mut result = std::mem::take(&mut *global);
        result.extend(std::mem::take(&mut self.singleton_dependencies));
        result.sort();
        result.dedup();
        result
    }

    /// Creates a [`RouterContext`] pre-populated with an OpenAPI spec.
    ///
    /// The spec is wrapped in an [`OpenApiRouter`] so that handlers can
    /// register their paths and schemas into it as they are visited.
    ///
    /// Only available when the `enable-open-api` feature is enabled.
    #[cfg(feature = "enable-open-api")]
    pub fn with_openapi(openapi: next_web_open_api::openapi::OpenApi) -> Self {
        let mut ctx = Self::default();
        ctx.open_api = Some(OpenApiRouter::with_openapi(openapi));
        ctx
    }
}

impl Default for RouterContext {
    fn default() -> Self {
        Self::new()
    }
}
