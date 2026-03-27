/// A simple data object keeping exitpoint related configs in a same place.
///
/// This struct encapsulates configuration data for an exit transition in a
/// state machine, which represents a special transition used to exit from a
/// composite state to outside.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[derive(Debug, Clone)]
pub struct ExitData<S, E> {
    /// Source state within the composite state
    source: S,
    /// Target exit point (typically a pseudo-state)
    target: S,

    _marker: std::marker::PhantomData<E>,
}

impl<S, E> ExitData<S, E> {
    /// Creates a new exit data with source and target.
    ///
    /// # Arguments
    /// * `source` - Source state
    /// * `target` - Target exit point
    ///
    /// # Returns
    /// A new `ExitData` instance
    pub fn new(source: S, target: S) -> Self {
        Self {
            source,
            target,
            _marker: std::marker::PhantomData,
        }
    }

    /// Gets the source state.
    ///
    /// # Returns
    /// The source state
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Gets the target exit point.
    ///
    /// # Returns
    /// The target exit point
    pub fn target(&self) -> &S {
        &self.target
    }
}

impl<S, E> Default for ExitData<S, E>
where
    S: Default + Clone + Eq,
{
    fn default() -> Self {
        Self {
            source: S::default(),
            target: S::default(),
            _marker: std::marker::PhantomData,
        }
    }
}
