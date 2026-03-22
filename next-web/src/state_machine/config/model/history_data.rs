use std::marker::PhantomData;

/// A simple data object keeping history related configs in a same place.
///
/// This struct encapsulates configuration data for a history transition in a
/// state machine, which represents the ability to remember and return to the
/// last active substate of a composite state.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[derive(Debug, Clone)]
pub struct HistoryData<S, E> {
    /// Source state (composite state with history)
    source: S,
    /// Target state (remembered history state)
    target: S,

    _marker: PhantomData<E>,
}

impl<S, E> HistoryData<S, E>
where
    S: Clone + Eq,
{
    /// Creates a new history data with source and target.
    ///
    /// # Arguments
    /// * `source` - Source state (composite state)
    /// * `target` - Target state (history state)
    ///
    /// # Returns
    /// A new `HistoryData` instance
    pub fn new(source: S, target: S) -> Self {
        Self {
            source,
            target,
            _marker: PhantomData,
        }
    }

    /// Gets the source state (composite state with history).
    ///
    /// # Returns
    /// The source state
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Gets the target state (remembered history state).
    ///
    /// # Returns
    /// The target state
    pub fn target(&self) -> &S {
        &self.target
    }
}

impl<S, E> Default for HistoryData<S, E>
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
