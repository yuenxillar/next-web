use crate::state_machine::config::model::state_data::StateData;

/// Data object used to return and build data from a `StateConfigurer`.
///
/// This struct holds a collection of state data objects that define
/// the states and their configurations in a state machine.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[derive(Clone)]
pub struct StatesData<S, E> {
    /// Collection of state data objects
    state_data: Vec<StateData<S, E>>,
}

impl<S, E> StatesData<S, E> {
    /// Creates a new states data container.
    ///
    /// # Arguments
    /// * `state_data` - Collection of state data objects
    ///
    /// # Returns
    /// A new `StatesData` instance containing the provided state data
    pub fn new(state_data: Vec<StateData<S, E>>) -> Self {
        Self { state_data }
    }

    /// Gets the state data collection.
    ///
    /// # Returns
    /// A reference to the collection of state data objects
    pub fn state_data(&self) -> &[StateData<S, E>] {
        &self.state_data
    }

    /// Gets a mutable reference to the state data collection.
    ///
    /// # Returns
    /// A mutable reference to the collection of state data objects
    pub fn state_data_mut(&mut self) -> &mut Vec<StateData<S, E>> {
        &mut self.state_data
    }

    /// Consumes the `StatesData` and returns the inner state data collection.
    ///
    /// # Returns
    /// The owned collection of state data objects
    pub fn into_state_data(self) -> Vec<StateData<S, E>> {
        self.state_data
    }

    /// Checks if the states data collection is empty.
    ///
    /// # Returns
    /// `true` if there are no state data objects, `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.state_data.is_empty()
    }

    /// Gets the number of state data objects in the collection.
    ///
    /// # Returns
    /// The count of state data objects
    pub fn len(&self) -> usize {
        self.state_data.len()
    }
}

impl<T, S, E> From<T> for StatesData<S, E>
where
    T: IntoIterator<Item = StateData<S, E>>,
{
    /// Creates a `StatesData` instance from a vector of state data.
    ///
    /// # Arguments
    /// * `state_data` - Vector of state data objects
    ///
    /// # Returns
    /// A new `StatesData` instance
    fn from(state_data: T) -> Self {
        Self {
            state_data: state_data.into_iter().collect(),
        }
    }
}

impl<S, E> Default for StatesData<S, E> {
    /// Creates an empty `StatesData` instance.
    ///
    /// # Returns
    /// A new `StatesData` with an empty state data collection
    fn default() -> Self {
        Self {
            state_data: Vec::new(),
        }
    }
}
