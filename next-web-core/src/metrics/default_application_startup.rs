//! Default "no op" [`ApplicationStartup`] implementation.
//!
//! This variant is designed for minimal overhead and does not record events.

use std::sync::OnceLock;

use super::ApplicationStartup;
use super::{StartupStep, Tags};

/// Default "no op" [`ApplicationStartup`] implementation.
///
/// This variant is designed for minimal overhead and does not record events.
///
/// # Shared Step
///
/// All calls to [`Self::start`] return the same shared
/// [`DefaultStartupStep`], avoiding any allocation per step.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultApplicationStartup;

impl DefaultApplicationStartup {
    /// Creates a new [`DefaultApplicationStartup`].
    ///
    /// # Returns
    ///
    /// A new no-op startup instance.
    pub fn new() -> Self {
        Self
    }
}

/// The shared no-op step returned by [`DefaultApplicationStartup`].
///
/// This step is a zero-cost singleton; it records nothing and all operations
/// are no-ops.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultStartupStep;

impl DefaultStartupStep {
    /// Creates a new [`DefaultStartupStep`].
    ///
    /// # Returns
    ///
    /// A new no-op step.
    pub fn new() -> Self {
        Self
    }

    /// Returns the shared singleton instance of this step.
    ///
    /// # Returns
    ///
    /// A reference to the shared step.
    pub fn shared() -> &'static DefaultStartupStep {
        static INSTANCE: OnceLock<DefaultStartupStep> = OnceLock::new();
        INSTANCE.get_or_init(DefaultStartupStep::new)
    }
}

impl StartupStep for DefaultStartupStep {
    fn name(&self) -> &str {
        "default"
    }

    fn id(&self) -> u64 {
        0
    }

    fn parent_id(&self) -> Option<u64> {
        None
    }

    fn tag(&mut self, _key: &str, _value: String) -> &mut dyn StartupStep {
        self
    }

    fn tag_lazy(&mut self, _key: &str, _value: &dyn FnOnce() -> String) -> &mut dyn StartupStep {
        // Do not invoke the supplier; no-op steps record nothing.
        self
    }

    fn tags(&self) -> Tags<'_> {
        Tags::empty()
    }

    fn end(&mut self) {}
}

impl ApplicationStartup for DefaultApplicationStartup {
    fn start(&self, _name: &str) -> Box<dyn StartupStep> {
        // Return the shared singleton, boxed. In a no-op implementation the
        // allocation is unavoidable with the current API, but the step itself
        // is a zero-sized type.
        Box::new(*DefaultStartupStep::shared())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_startup_returns_step() {
        let startup = DefaultApplicationStartup::new();
        let step = startup.start("any.name");
        assert_eq!(step.name(), "default");
    }

    #[test]
    fn default_startup_step_id_is_zero() {
        let startup = DefaultApplicationStartup::new();
        let step = startup.start("any.name");
        assert_eq!(step.id(), 0);
    }

    #[test]
    fn default_startup_step_has_no_parent() {
        let startup = DefaultApplicationStartup::new();
        let step = startup.start("any.name");
        assert_eq!(step.parent_id(), None);
    }

    #[test]
    fn default_startup_step_has_empty_tags() {
        let startup = DefaultApplicationStartup::new();
        let step = startup.start("any.name");
        assert!(step.tags().is_empty());
    }

    #[test]
    fn default_startup_tag_is_no_op() {
        let startup = DefaultApplicationStartup::new();
        let mut step = startup.start("any.name");
        step.tag("key", "value".to_string());
        assert!(step.tags().is_empty());
    }

    #[test]
    fn default_startup_tag_lazy_does_not_invoke_supplier() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let startup = DefaultApplicationStartup::new();
        let mut step = startup.start("any.name");
        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = Arc::clone(&invoked);

        step.tag_lazy("key", &|| {
            invoked_clone.store(true, Ordering::Relaxed);
            "value".to_string()
        });

        assert!(!invoked.load(Ordering::Relaxed));
    }

    #[test]
    fn default_startup_end_is_no_op() {
        let startup = DefaultApplicationStartup::new();
        let mut step = startup.start("any.name");
        step.end();
        step.end(); // idempotent
    }

    #[test]
    fn shared_step_is_singleton() {
        let a = DefaultStartupStep::shared();
        let b = DefaultStartupStep::shared();
        assert!(std::ptr::eq(a, b));
    }

    #[test]
    fn default_application_startup_is_default() {
        let startup = DefaultApplicationStartup::default();
        let step = startup.start("any.name");
        assert_eq!(step.name(), "default");
    }
}
