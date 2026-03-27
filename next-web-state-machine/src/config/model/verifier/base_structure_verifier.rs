use std::{fmt::Debug, marker::PhantomData};

use next_web_core::error::BoxError;

use crate::config::model::{
    state_data::StateData, state_machine_model::StateMachineModel,
    verifier::state_machine_model_verifier::StateMachineModelVerifier,
};

#[derive(Debug)]
pub struct BaseStructureVerifier<S, E>(PhantomData<(S, E)>);

impl<S, E> BaseStructureVerifier<S, E>
where
    S: Debug + PartialEq + Send + Sync + 'static,
    E: Send + Sync,
{
    fn verify_state_group(
        &self,
        states: &[StateData<S, E>],
        parent: Option<&S>,
    ) -> Result<(), BoxError> {
        let children = self.collect_children(states, parent)?;

        if children.is_empty() {
            if parent.is_none() {
                return Err("Initial state not set".into());
            }

            return Ok(());
        }

        if !children.iter().any(|state| state.is_initial()) {
            return Err(self.initial_state_not_set_error(parent, &children));
        }

        for child in children {
            self.verify_state_group(states, Some(child.state()))?;
        }

        Ok(())
    }

    fn collect_children<'a>(
        &self,
        states: &'a [StateData<S, E>],
        parent: Option<&S>,
    ) -> Result<Vec<&'a StateData<S, E>>, BoxError> {
        let mut children = Vec::new();

        for state_data in states {
            if self.matches_parent(state_data, parent)? {
                children.push(state_data);
            }
        }

        Ok(children)
    }

    fn matches_parent(
        &self,
        state_data: &StateData<S, E>,
        expected_parent: Option<&S>,
    ) -> Result<bool, BoxError> {
        match (state_data.parent(), expected_parent) {
            (None, None) => Ok(true),
            (None, Some(_)) => Ok(false),
            (Some(_), None) => Ok(false),
            (Some(parent), Some(expected_parent)) => {
                let actual_parent = parent.downcast_ref::<S>().ok_or_else(|| -> BoxError {
                    format!(
                        "State parent type mismatch for state {:?}",
                        state_data.state()
                    )
                    .into()
                })?;

                Ok(actual_parent == expected_parent)
            }
        }
    }

    fn initial_state_not_set_error(
        &self,
        parent: Option<&S>,
        children: &[&StateData<S, E>],
    ) -> BoxError {
        let scope = parent
            .map(|parent| format!("parent state {:?}", parent))
            .unwrap_or_else(|| "top level".to_string());

        let trace = children
            .iter()
            .map(|child| format!("state={:?}, region={:?}", child.state(), child.region()))
            .collect::<Vec<_>>()
            .join("; ");

        format!("Initial state not set for {}. {}", scope, trace).into()
    }
}

impl<S, E> StateMachineModelVerifier<S, E> for BaseStructureVerifier<S, E>
where
    S: Debug + PartialEq + Send + Sync + 'static,
    E: Send + Sync,
{
    fn verify(&self, model: &dyn StateMachineModel<S, E>) -> Result<(), BoxError> {
        if model
            .get_transitions_data()
            .map(|transitions| transitions.transitions().is_empty())
            .unwrap_or(true)
        {
            return Err("Must have at least one transition".into());
        }

        let states = model
            .get_states_data()
            .map(|states| states.state_data())
            .unwrap_or(&[]);

        self.verify_state_group(states, None)
    }
}

impl<S, E> Default for BaseStructureVerifier<S, E> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use std::{any::Any, sync::Arc};

    use crate::config::model::{
        configuration_data::ConfigurationData,
        default_state_machine_model::DefaultStateMachineModel, state_data::StateData,
        transition_data::TransitionData, transitions_data::TransitionsData,
        verifier::state_machine_model_verifier::StateMachineModelVerifier,
    };

    use super::BaseStructureVerifier;

    #[test]
    fn rejects_missing_top_level_initial_state() {
        let verifier = BaseStructureVerifier::<i32, i32>::default();
        let model = DefaultStateMachineModel::new(
            ConfigurationData::default(),
            Some(vec![StateData::new(1), StateData::new(2)].into()),
            Some(TransitionsData::new(vec![TransitionData::new(1, 2, 99)])),
        );

        let error = verifier.verify(&model).unwrap_err().to_string();

        assert!(error.contains("Initial state not set for top level"));
    }

    #[test]
    fn rejects_missing_initial_state_for_child_group() {
        let verifier = BaseStructureVerifier::<i32, i32>::default();

        let mut root = StateData::new(1);
        root.set_initial(true);

        let child_a = StateData::with_hierarchy(Some(Arc::new(1) as Arc<dyn Any>), None, 10, false);
        let child_b = StateData::with_hierarchy(Some(Arc::new(1) as Arc<dyn Any>), None, 11, false);

        let model = DefaultStateMachineModel::new(
            ConfigurationData::default(),
            Some(vec![root, child_a, child_b].into()),
            Some(TransitionsData::new(vec![TransitionData::new(1, 10, 99)])),
        );

        let error = verifier.verify(&model).unwrap_err().to_string();

        assert!(error.contains("parent state 1"));
    }
}
