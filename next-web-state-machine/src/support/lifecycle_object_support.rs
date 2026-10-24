use next_web_core::{async_trait, error::BoxError};

/// Support object carrying the lifecycle hooks of a machine component.
///
/// The struct itself does nothing; the hooks live in
/// [`LifecycleObjectSupportExt`], whose default implementations succeed.
#[derive(Clone, Default)]
pub struct LifecycleObjectSupport {}

impl LifecycleObjectSupport {
    /// Creates a new lifecycle support object.
    pub fn new() -> Self {
        Self {}
    }

    /// Runs the initialisation hook.
    pub fn on_init(&mut self) -> Result<(), BoxError> {
        Ok(())
    }
}

/// Lifecycle hooks of a machine component.
#[async_trait]
pub trait LifecycleObjectSupportExt {
    /// Called once, before the component is used.
    fn on_init(&mut self) -> Result<(), BoxError> {
        Ok(())
    }

    /// Called when the component is destroyed.
    async fn do_destroy(&self) {}

    /// Called before the component starts.
    async fn do_pre_start(&mut self) {}

    /// Called before the component stops.
    async fn do_pre_stop(&mut self) {}

    /// Called after the component started.
    async fn do_post_start(&self) {}
}
