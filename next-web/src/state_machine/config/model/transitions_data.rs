use std::collections::HashMap;

use crate::state_machine::config::model::{
    choice_data::ChoiceData, entry_data::EntryData, exit_data::ExitData, history_data::HistoryData,
    junction_data::JunctionData, transition_data::TransitionData,
};

/// Data object for transitions.
///
/// This struct encapsulates all transition-related data for a state machine,
/// including regular transitions, choice points, junctions, forks, joins,
/// entry/exit transitions, and history transitions.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub struct TransitionsData<S, E> {
    /// Collection of regular transition data
    transitions: Vec<TransitionData<S, E>>,
    /// Mapping from source states to their choice points
    choices: Option<HashMap<S, Vec<ChoiceData<S, E>>>>,
    /// Mapping from source states to their junction points
    junctions: Option<HashMap<S, Vec<JunctionData<S, E>>>>,
    /// Mapping from fork states to target states
    forks: Option<HashMap<S, Vec<S>>>,
    /// Mapping from join states to target states
    joins: Option<HashMap<S, Vec<S>>>,
    /// Collection of entry transition data
    entrys: Option<Vec<EntryData<S, E>>>,
    /// Collection of exit transition data
    exits: Option<Vec<ExitData<S, E>>>,
    /// Collection of history transition data
    historys: Option<Vec<HistoryData<S, E>>>,
}

impl<S, E> TransitionsData<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    /// Creates a new transitions data object with only regular transitions.
    ///
    /// # Arguments
    /// * `transitions_data` - Collection of transition data
    ///
    /// # Returns
    /// A new `TransitionsData` instance
    pub fn new(transitions_data: Vec<TransitionData<S, E>>) -> Self {
        Self::with_extended(transitions_data, None, None, None, None, None, None, None)
    }

    /// Creates a new transitions data object with all transition types.
    ///
    /// # Arguments
    /// * `transitions_data` - Collection of transition data
    /// * `choices` - Mapping from source states to choice points
    /// * `junctions` - Mapping from source states to junction points
    /// * `forks` - Mapping from fork states to target states
    /// * `joins` - Mapping from join states to target states
    /// * `entrys` - Collection of entry transition data
    /// * `exits` - Collection of exit transition data
    /// * `historys` - Collection of history transition data
    ///
    /// # Returns
    /// A new `TransitionsData` instance
    pub fn with_extended(
        transitions_data: Vec<TransitionData<S, E>>,
        choices: Option<HashMap<S, Vec<ChoiceData<S, E>>>>,
        junctions: Option<HashMap<S, Vec<JunctionData<S, E>>>>,
        forks: Option<HashMap<S, Vec<S>>>,
        joins: Option<HashMap<S, Vec<S>>>,
        entrys: Option<Vec<EntryData<S, E>>>,
        exits: Option<Vec<ExitData<S, E>>>,
        historys: Option<Vec<HistoryData<S, E>>>,
    ) -> Self {
        Self {
            transitions: transitions_data,
            choices,
            junctions,
            forks,
            joins,
            entrys,
            exits,
            historys,
        }
    }

    /// Gets the collection of regular transitions.
    ///
    /// # Returns
    /// A reference to the transitions collection
    pub fn transitions(&self) -> &[TransitionData<S, E>] {
        &self.transitions
    }

    /// Gets a mutable reference to the collection of regular transitions.
    ///
    /// # Returns
    /// A mutable reference to the transitions collection
    pub fn transitions_mut(&mut self) -> &mut Vec<TransitionData<S, E>> {
        &mut self.transitions
    }

    /// Gets the choices mapping.
    ///
    /// # Returns
    /// A reference to the choices mapping if present
    pub fn choices(&self) -> Option<&HashMap<S, Vec<ChoiceData<S, E>>>> {
        self.choices.as_ref()
    }

    /// Gets a mutable reference to the choices mapping.
    ///
    /// # Returns
    /// A mutable reference to the choices mapping if present
    pub fn choices_mut(&mut self) -> Option<&mut HashMap<S, Vec<ChoiceData<S, E>>>> {
        self.choices.as_mut()
    }

    /// Gets the junctions mapping.
    ///
    /// # Returns
    /// A reference to the junctions mapping if present
    pub fn junctions(&self) -> Option<&HashMap<S, Vec<JunctionData<S, E>>>> {
        self.junctions.as_ref()
    }

    /// Gets a mutable reference to the junctions mapping.
    ///
    /// # Returns
    /// A mutable reference to the junctions mapping if present
    pub fn junctions_mut(&mut self) -> Option<&mut HashMap<S, Vec<JunctionData<S, E>>>> {
        self.junctions.as_mut()
    }

    /// Gets the forks mapping.
    ///
    /// # Returns
    /// A reference to the forks mapping if present
    pub fn forks(&self) -> Option<&HashMap<S, Vec<S>>> {
        self.forks.as_ref()
    }

    /// Gets a mutable reference to the forks mapping.
    ///
    /// # Returns
    /// A mutable reference to the forks mapping if present
    pub fn forks_mut(&mut self) -> Option<&mut HashMap<S, Vec<S>>> {
        self.forks.as_mut()
    }

    /// Gets the joins mapping.
    ///
    /// # Returns
    /// A reference to the joins mapping if present
    pub fn joins(&self) -> Option<&HashMap<S, Vec<S>>> {
        self.joins.as_ref()
    }

    /// Gets a mutable reference to the joins mapping.
    ///
    /// # Returns
    /// A mutable reference to the joins mapping if present
    pub fn joins_mut(&mut self) -> Option<&mut HashMap<S, Vec<S>>> {
        self.joins.as_mut()
    }

    /// Gets the collection of entry transitions.
    ///
    /// # Returns
    /// A reference to the entry transitions collection if present
    pub fn entrys(&self) -> Option<&[EntryData<S, E>]> {
        self.entrys.as_deref()
    }

    /// Gets a mutable reference to the collection of entry transitions.
    ///
    /// # Returns
    /// A mutable reference to the entry transitions collection if present
    pub fn entrys_mut(&mut self) -> Option<&mut Vec<EntryData<S, E>>> {
        self.entrys.as_mut()
    }

    /// Gets the collection of exit transitions.
    ///
    /// # Returns
    /// A reference to the exit transitions collection if present
    pub fn exits(&self) -> Option<&[ExitData<S, E>]> {
        self.exits.as_deref()
    }

    /// Gets a mutable reference to the collection of exit transitions.
    ///
    /// # Returns
    /// A mutable reference to the exit transitions collection if present
    pub fn exits_mut(&mut self) -> Option<&mut Vec<ExitData<S, E>>> {
        self.exits.as_mut()
    }

    /// Gets the collection of history transitions.
    ///
    /// # Returns
    /// A reference to the history transitions collection if present
    pub fn historys(&self) -> Option<&[HistoryData<S, E>]> {
        self.historys.as_deref()
    }

    /// Gets a mutable reference to the collection of history transitions.
    ///
    /// # Returns
    /// A mutable reference to the history transitions collection if present
    pub fn historys_mut(&mut self) -> Option<&mut Vec<HistoryData<S, E>>> {
        self.historys.as_mut()
    }

    /// Checks if the transitions data is empty.
    ///
    /// # Returns
    /// `true` if there are no transitions, choices, junctions, forks,
    /// joins, entrys, exits, or historys, `false` otherwise
    pub fn is_empty(&self) -> bool {
        self.transitions.is_empty()
            && self.choices.as_ref().map(|c| c.is_empty()).unwrap_or(true)
            && self
                .junctions
                .as_ref()
                .map(|j| j.is_empty())
                .unwrap_or(true)
            && self.forks.as_ref().map(|f| f.is_empty()).unwrap_or(true)
            && self.joins.as_ref().map(|j| j.is_empty()).unwrap_or(true)
            && self.entrys.as_ref().map(|e| e.is_empty()).unwrap_or(true)
            && self.exits.as_ref().map(|e| e.is_empty()).unwrap_or(true)
            && self.historys.as_ref().map(|h| h.is_empty()).unwrap_or(true)
    }

    /// Gets the total count of all transitions.
    ///
    /// # Returns
    /// The total number of all transition objects
    pub fn count(&self) -> usize {
        let mut total = self.transitions.len();

        if let Some(choices) = &self.choices {
            total += choices.values().map(|v| v.len()).sum::<usize>();
        }

        if let Some(junctions) = &self.junctions {
            total += junctions.values().map(|v| v.len()).sum::<usize>();
        }

        if let Some(forks) = &self.forks {
            total += forks.values().map(|v| v.len()).sum::<usize>();
        }

        if let Some(joins) = &self.joins {
            total += joins.values().map(|v| v.len()).sum::<usize>();
        }

        if let Some(entrys) = &self.entrys {
            total += entrys.len();
        }

        if let Some(exits) = &self.exits {
            total += exits.len();
        }

        if let Some(historys) = &self.historys {
            total += historys.len();
        }

        total
    }
}

impl<S, E> Default for TransitionsData<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    /// Creates an empty `TransitionsData` instance.
    ///
    /// # Returns
    /// A new `TransitionsData` with empty collections
    fn default() -> Self {
        Self {
            transitions: Vec::new(),
            choices: None,
            junctions: None,
            forks: None,
            joins: None,
            entrys: None,
            exits: None,
            historys: None,
        }
    }
}

impl<S, E> Clone for TransitionsData<S, E>
where
    S: Clone,
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            transitions: self.transitions.clone(),
            choices: self.choices.clone(),
            junctions: self.junctions.clone(),
            forks: self.forks.clone(),
            joins: self.joins.clone(),
            entrys: self.entrys.clone(),
            exits: self.exits.clone(),
            historys: self.historys.clone(),
        }
    }
}
