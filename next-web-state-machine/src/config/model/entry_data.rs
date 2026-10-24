use std::marker::PhantomData;

/// A simple data object keeping entrypoint related configs in a same place.
///
/// This struct encapsulates configuration data for an entry transition in a
/// state machine, which represents a special transition used to enter a
/// composite state from outside.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[derive(Debug, Clone)]
pub struct EntryData<S, E> {
    /// Source entry point (typically a pseudo-state)
    source: S,
    /// Target state within the composite state
    target: S,

    _marker: PhantomData<E>,
}

impl<S, E> EntryData<S, E>
where
    S: Clone + Eq,
{
    /// Creates a new entry data with source and target.
    ///
    /// # Arguments
    /// * `source` - Source entry point
    /// * `target` - Target state
    ///
    /// # Returns
    /// A new `EntryData` instance
    pub fn new(source: S, target: S) -> Self {
        Self {
            source,
            target,
            _marker: PhantomData,
        }
    }

    /// Gets the source entry point.
    ///
    /// # Returns
    /// The source entry point
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Gets the target state.
    ///
    /// # Returns
    /// The target state
    pub fn target(&self) -> &S {
        &self.target
    }
}

impl<S, E> Default for EntryData<S, E>
where
    S: Default + Clone + Eq,
{
    fn default() -> Self {
        Self {
            source: S::default(),
            target: S::default(),
            _marker: PhantomData,
        }
    }
}
