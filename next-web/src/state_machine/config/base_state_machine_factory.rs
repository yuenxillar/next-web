use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

use next_web_core::error::BoxError;
use uuid::Uuid;

use crate::state_machine::config::action::StateMachineAction;
use crate::state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use crate::state_machine::config::model::state_data::StateData;
use crate::state_machine::config::model::state_machine_model::StateMachineModel;
use crate::state_machine::config::model::state_machine_model_factory::StateMachineModelFactory;
use crate::state_machine::config::model::transition_data::TransitionData;
use crate::state_machine::config::model::transitions_data::TransitionsData;
use crate::state_machine::config::model::verifier::base_structure_verifier::Tree;
use crate::state_machine::config::model::verifier::composite_state_machine_model_verifier::CompositeStateMachineModelVerifier;
use crate::state_machine::config::model::verifier::state_machine_model_verifier::StateMachineModelVerifier;
use crate::state_machine::config::state_machine_factory::StateMachineFactory;
use crate::state_machine::extended_state::ExtendedState;
use crate::state_machine::listener::state_machine_listener::StateMachineListener;
use crate::state_machine::monitor::state_machine_monitor::StateMachineMonitor;
use crate::state_machine::region::Region;
use crate::state_machine::state::default_pseudo_state::DefaultPseudoState;
use crate::state_machine::state::pseudo_state::PseudoState;
use crate::state_machine::state::pseudo_state_kind::PseudoStateKind;
use crate::state_machine::state::StateMachineState;
use crate::state_machine::support::default_extended_state::DefaultExtendedState;
use crate::state_machine::support::state_machine_interceptor::StateMachineInterceptor;
use crate::state_machine::transition::default_local_transition::DefaultLocalTransition;
use crate::state_machine::transition::initial_transition::InitialTransition;
use crate::state_machine::transition::transition_kind::TransitionKind;
use crate::state_machine::transition::StateMachineTransition;
use crate::state_machine::trigger::Trigger;
use crate::state_machine::StateMachine;

// A placeholder for the internal stack item used during machine building.
// This might be better represented as a tuple or integrated differently depending on the full implementation.
#[derive(Clone)]
struct MachineStackItem<S, E> {
    machine: Arc<dyn StateMachine<S, E>>, // Assuming StateMachine is boxed for dynamic dispatch if needed
    // Add other fields if necessary based on Java logic
    _phantom: PhantomData<(S, E)>, // Placeholder if S, E are used internally but not stored
}

// A placeholder for the holder list item used during machine building.
// This might be better represented as a tuple or integrated differently depending on the full implementation.
#[derive(Debug)]
struct HolderListItem<S> {
    key: S,
    value: Box<dyn std::any::Any>,
}

///  base factory for creating state machines.
pub struct BaseStateMachineFactory<S, E> {
    default_state_machine_model: Arc<dyn StateMachineModel<S, E>>,
    state_machine_model_factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,
    context_events: Option<bool>,
    handle_autostartup: bool,
    name: String,
    default_state_machine_monitor: Option<Arc<dyn StateMachineMonitor<S, E>>>,
}

impl<S, E> BaseStateMachineFactory<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Clone,
    E: Clone,
{
    /// Creates a new instance of the factory.
    pub fn new(
        default_state_machine_model: Arc<dyn StateMachineModel<S, E>>,
        state_machine_model_factory: Option<Arc<dyn StateMachineModelFactory<S, E>>>,
    ) -> Self {
        Self {
            default_state_machine_model,
            state_machine_model_factory,
            context_events: Default::default(),
            name: String::from("stateMachine"),
            handle_autostartup: Default::default(),
            default_state_machine_monitor: Default::default(),
        }
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Builds a state machine based on the provided model.
    /// This is the main logic extracted from the Java `getStateMachine` method.
    fn _get_state_machine(
        &self,
        uuid: Option<uuid::Uuid>,
        machine_id: Option<String>,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>
    where
        S: Debug,
    {
        // Use your specific error type
        let mut machines = Vec::<Box<dyn StateMachine<S, E>>>::new();
        let state_machine_model = self.resolve_state_machine_model(machine_id.as_deref());

        // Verify the model if verification is enabled.
        if state_machine_model
            .get_configuration_data()
            .is_verifier_enabled()
        {
            let mut verifier = state_machine_model.get_configuration_data().verifier();
            match verifier {
                Some(verifier) => verifier.verify(state_machine_model.as_ref())?,
                None => {
                    // For now, assume a verifier exists or is disabled if None.
                    // If it's truly optional and None, verification is skipped.
                    CompositeStateMachineModelVerifier::default()
                        .verify(state_machine_model.as_ref())?;
                }
            };
        }

        // SHARED
        let default_extended_state = DefaultExtendedState::default();

        let mut machine: Option<Arc<dyn StateMachine<S, E>>> = None;

        // we store mappings from state id's to states which gets
        // created during the process. This is needed for transitions to
        // find a correct mappings because they use state id's, not actual
        // states.
        let mut state_map: HashMap<S, Box<dyn StateMachineState<S, E>>> = HashMap::new();
        let mut region_stack: Vec<MachineStackItem<S, E>> = Vec::new();
        let mut state_stack: Vec<StateData<S, E>> = Vec::new(); // Stack of StateData during traversal
        let mut machine_map: HashMap<S, Arc<dyn StateMachine<S, E>>> = HashMap::new(); // Parent state ID -> Submachine
        let mut holder_list: Vec<HolderListItem<S>> = Vec::new(); // Holders for states created early

        // Build the state hierarchy tree and iterate it post-order.
        let iterator = self.build_state_data_iterator(state_machine_model);

        // The main loop from Java's getStateMachine is complex. Here's a simplified representation
        // of how the tree iterator drives the machine building process.
        // Actual implementation of `buildMachine` and related functions is needed.
        for node in iterator {
            let state_data = node.data(); // Gets the StateData<S, E> from the tree node
            let peek = state_stack.last(); // Peeks at the last StateData pushed onto the stack

            // --- Core Logic from Java getStateMachine Loop (Simplified) ---
            // This part handles pushing/popping the state_stack based on parent-child relationships,
            // grouping states by their common parent, and then calling buildMachine.
            // The Java code uses a while loop with `iterator.hasNext()`, but Rust iterators are different.
            // We'll simulate the logic by processing each node and managing the stack manually if needed.

            // Push initial state data onto the stack
            if state_stack.is_empty() {
                state_stack.push(state_data.clone());
                continue;
            }

            // Determine if the current state's parent matches the parent of the state on top of the stack
            let top_of_stack_parent = state_stack.last().unwrap().parent();
            let current_state_parent = state_data.parent();

            if current_state_parent.as_ref() == top_of_stack_parent.as_ref() {
                // Same parent, add to current group being processed
                state_stack.push(state_data.clone());
            } else {
                // Different parent, means the previous group is complete.
                // Pop all items with the same parent from the stack.
                let popped_states = pop_same_parents(&mut state_stack);

                // Process the popped group of states (e.g., build a machine for a region)
                let initial_count = get_initial_count(&popped_states);
                let regions_state_datas = split_into_regions(&popped_states);

                // Determine transitions relevant to this group
                // This logic depends on whether there are more nodes in the iterator (roots = hasNext())
                // For simplicity here, assume we get transitions for the popped states.
                let transitions_data = self.resolve_transition_data(
                    state_machine_model.get_transitions_data().get_transitions(),
                    &popped_states,
                );

                if initial_count > 1 {
                    // Handle regions: build a machine for each region
                    for region_state_datas in regions_state_datas {
                        // Build machine for this region
                        let region_machine = self.build_machine(
                            &mut machine_map,
                            &mut state_map,
                            &mut holder_list,
                            &region_state_datas,
                            &transitions_data,
                            // context_events, // Assuming context_events is handled via config
                            default_extended_state,
                            state_machine_model.get_transitions_data(),
                            machine_id.as_deref(),
                            uuid,
                            state_machine_model,
                        )?;
                        region_stack.push(MachineStackItem {
                            machine: region_machine,
                            _phantom: PhantomData,
                        });
                        machines.push(region_stack.last().unwrap().machine);
                        // Assuming box_clone for StateMachine trait object
                    }

                    // Collect regions and build a RegionState
                    let regions: Vec<Box<dyn Region<S, E>>> = region_stack
                        .drain(..)
                        .map(|item| item.machine.as_region())
                        .collect(); // Assuming as_region() method exists
                    if let Some(parent_data) = peek {
                        let parent_state_id = parent_data.parent().unwrap(); // Get parent ID from the state that defined this group
                        let region_state = self.build_region_state_internal(
                            parent_state_id,
                            regions,
                            // deferred_actions, entry_actions, exit_actions, pseudo_state, state_machine_model
                            // ... pass relevant arguments
                        )?;
                        // Add region state to map and potentially create a top-level machine
                        state_map.insert(
                            state_data.state().clone(),
                            region_state.box_clone_as_state(),
                        ); // Assuming box_clone_as_state method exists
                    }
                } else {
                    // Single machine/group
                    machine = Some(self.build_machine(
                        &mut machine_map,
                        &mut state_map,
                        &mut holder_list,
                        &popped_states,
                        &transitions_data,
                        // context_events,
                        default_extended_state,
                        state_machine_model.get_transitions_data(),
                        machine_id.as_deref(),
                        uuid,
                        state_machine_model,
                    )?);
                    machines.push(machine.as_ref().unwrap().box_clone()); // Assuming box_clone for StateMachine trait object

                    // Add to machine map if it's an initial state or not already mapped
                    if let Some(p) = peek {
                        if p.is_initial() || !machine_map.contains_key(p.parent().as_ref().unwrap())
                        {
                            machine_map.insert(
                                p.parent().unwrap().clone(),
                                machine.as_ref().unwrap().box_clone(),
                            ); // Assuming box_clone
                        }
                    }
                }

                // Push the current state data onto the stack to start a new potential group
                state_stack.push(state_data.clone());
            }
        }

        // Finalize the last group if the stack isn't empty
        if !state_stack.is_empty() {
            let final_popped_states = pop_same_parents(&mut state_stack);
            // Process final_popped_states similarly...
            if !final_popped_states.is_empty() {
                let transitions_data = self.resolve_transition_data(
                    state_machine_model.get_transitions_data().get_transitions(),
                    &final_popped_states,
                );
                machine = Some(self.build_machine(
                    &mut machine_map,
                    &mut state_map,
                    &mut holder_list,
                    &final_popped_states,
                    &transitions_data,
                    // context_events,
                    default_extended_state,
                    state_machine_model.get_transitions_data(),
                    machine_id.as_deref(),
                    uuid,
                    state_machine_model,
                )?);
                machines.push(machine.as_ref().unwrap().box_clone()); // Assuming box_clone
            }
        }

        // --- Post-Build Setup (Simplified from Java) ---
        let final_machine = machine.ok_or("Failed to build any state machine")?;

        // Setup autostartup (if applicable)
        // if let Some(lifecycle_support) = final_machine.as_lifecycle_object_support() {
        //     lifecycle_support.set_auto_startup(state_machine_model.get_configuration_data().is_auto_start());
        // }

        // Set top-level machine as relay (if applicable)
        // final_machine.get_state_machine_accessor().do_with_all_regions(|f| f.set_relay(final_machine));

        // Add monitoring (if applicable)
        // if let Some(monitor) = state_machine_monitor.or(self.default_state_machine_monitor.as_deref()) {
        //      final_machine.get_state_machine_accessor().do_with_region(|f| f.add_state_machine_monitor(monitor));
        // }

        // Set parent machines (if applicable)
        // for (parent_id, submachine) in &machine_map {
        //     let parent_machine = machine_map.get(parent_id.parent());
        //     submachine.get_state_machine_accessor().do_with_region(|f| f.set_parent_machine(parent_machine));
        // }

        // Init built machines (if applicable)
        // for m in &mut machines {
        //     m.after_properties_set()?;
        // }

        // Add security interceptor (if applicable)
        // if state_machine_model.get_configuration_data().is_security_enabled() {
        //      final_machine.get_state_machine_accessor().do_with_all_regions(|f| f.add_state_machine_interceptor(security_interceptor));
        // }

        // Setup distributed state machine (if applicable)
        // if let Some(ensemble) = state_machine_model.get_configuration_data().get_state_machine_ensemble() {
        //     let distributed_sm = DistributedStateMachine::new(ensemble, final_machine);
        //     // Configure distributed_sm...
        //     final_machine = Box::new(distributed_sm);
        // }

        // Add listeners (if applicable)
        // for listener in state_machine_model.get_configuration_data().get_state_machine_listeners() {
        //     final_machine.add_state_listener(listener);
        // }

        // Add interceptors (if applicable)
        // for interceptor in state_machine_model.get_configuration_data().get_state_machine_interceptors() {
        //     final_machine.get_state_machine_accessor().do_with_all_regions(|f| f.add_state_machine_interceptor(interceptor));
        // }

        // Fix state references in holders (if applicable)
        // for holder_item in &holder_list {
        //     holder_item.value.set_state(state_map.get(&holder_item.key));
        // }

        Ok(final_machine)
    }

    /// Builds an iterator for traversing the state data hierarchy in post-order.
    /// This corresponds to the Java `buildStateDataIterator` method.
    fn build_state_data_iterator(
        &self,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> impl Iterator<Item = &TreeNode<StateData<S, E>>> {
        let mut tree: Tree<StateData<S, E>> = Tree::new();

        // Add states to the tree using the Tree utility, similar to Java.
        self.tree_add(
            &mut tree,
            state_machine_model.get_states_data().get_state_data(),
        );

        // Perform a post-order traversal on the tree and return an iterator.
        // This requires implementing or using a post-order iterator for the Tree struct.
        // Assuming Tree has a `post_order_traversal` method returning a Vec or similar.
        tree.post_order_traversal().into_iter()
    }

    /// Recursively adds state data to the tree, including submachine state data.
    fn tree_add(&self, tree: &mut Tree<StateData<S, E>>, state_datas: &[StateData<S, E>]) {
        if state_datas.is_empty() {
            return;
        }

        for state_data in state_datas {
            // Use the state's ID as the tree node ID and its parent ID as the parent.
            let id = state_data.state().clone();
            let parent_id = state_data.parent().clone();
            tree.add(state_data.clone(), id, parent_id);

            // Recursively add submachine state data if present.
            if let Some(submachine_state_data) = state_data.submachine_state_data() {
                self.tree_add(tree, submachine_state_data);
            }
        }
    }

    /// Resolves the state machine model to use, considering the factory and potential machine ID.
    /// This corresponds to the Java `resolveStateMachineModel` method.
    fn resolve_state_machine_model(
        &self,
        machine_id: Option<&str>,
    ) -> Arc<dyn StateMachineModel<S, E>> {
        let model_factory = match self.state_machine_model_factory.as_ref() {
            Some(model_factory) => model_factory,
            None => return self.default_state_machine_model.clone(),
        };

        model_factory.build_with_machine_id(machine_id.unwrap_or_default().to_string())

        // if m.get_configuration_data() {
        //     // if model doesn't have explicit configuration data,
        //     // get it from default model
        //     let configuration = self
        //         .default_state_machine_model
        //         .get_configuration_data()
        //         .map(Clone::clone)
        //         .unwrap();
        //     let states = m.get_states_data().clone();
        //     let transitions = m.get_transitions_data().clone();

        //     let model = DefaultStateMachineModel::new(configuration, states, transitions);
        //     Arc::new(model)
        // } else {
        //     m
        // }
    }

    /// Resolves transitions relevant to a specific set of states.
    /// This corresponds to the Java `resolveTransitionData` method.
    fn resolve_transition_data(
        &self,
        all_transitions: &[TransitionData<S, E>],
        state_datas: &[StateData<S, E>],
    ) -> Vec<TransitionData<S, E>> {
        let mut relevant_transitions = Vec::new();
        let state_ids: std::collections::HashSet<_> =
            state_datas.iter().map(|sd| sd.state()).collect();

        for transition_data in all_transitions {
            // Check if the transition's state (often source or target) is in the set of relevant states.
            // The exact logic depends on how TransitionData relates to StateData in your model.
            // This is a simplified check based on the state field potentially indicating scope.
            if let Some(transition_state) = transition_data.state() {
                // Assuming TransitionData has a state field
                if state_ids.contains(&transition_state) {
                    relevant_transitions.push(transition_data.clone());
                }
            }
        }
        relevant_transitions
    }

    /// Builds a single state machine for a group of states and transitions.
    /// This is a complex method stubbed here. Its full implementation depends heavily on the
    /// `State`, `Transition`, and `StateMachine` trait/object implementations.
    fn build_machine(
        &self,
        machine_map: &mut HashMap<S, Arc<dyn StateMachine<S, E>>>,
        state_map: &mut HashMap<S, Box<dyn StateMachineState<S, E>>>,
        holder_list: &mut Vec<HolderListItem<S>>,
        state_datas: &[StateData<S, E>],
        transitions_data: &[TransitionData<S, E>],
        // context_events: Option<bool>,
        default_extended_state: &dyn ExtendedState,
        state_machine_transitions: TransitionsData<S, E>,
        machine_id: Option<&str>,
        uuid: Option<uuid::Uuid>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Box<dyn StateMachine<S, E>>, Box<dyn std::error::Error + Send + Sync>> {
        // Use your specific error type
        // --- Core Building Logic (Stubbed) ---
        // 1. Create State objects from StateData
        let mut states = Vec::<Box<dyn StateMachineState<S, E>>>::new();
        let mut initial_state: Option<Box<dyn StateMachineState<S, E>>> = None;
        let mut initial_action: Option<Arc<dyn StateMachineAction<S, E>>> = None;

        for state_data in state_datas {
            // Check if state already exists in map (for submachines or reuse)
            if let Some(existing_state) = state_map.get(state_data.state()) {
                states.push(existing_state.box_clone()); // Assuming box_clone
                if state_data.is_initial() {
                    initial_state = Some(existing_state.box_clone()); // Assuming box_clone
                }
                continue;
            }

            // Determine if it's a submachine state
            let mut state: Box<dyn StateMachineState<S, E>>;
            if let Some(submachine) = state_data.submachine() {
                // Create StateMachineState
                let sm_state = StateMachineState::new(
                    state_data.state().clone(),
                    submachine.box_clone(), // Assuming box_clone for submachine
                    state_data.deferred().clone(),
                    state_data.entry_actions().clone(),
                    state_data.exit_actions().clone(),
                    // pseudo_state if initial
                    if state_data.is_initial() {
                        Some(Box::new(DefaultPseudoState::new(PseudoStateKind::Initial)))
                    } else {
                        None
                    },
                );
                state = Box::new(sm_state);
            } else {
                // Create regular state
                // Use state_data properties to build the state (entry/exit actions, pseudo-states, etc.)
                let pseudo_state = if state_data.is_initial() {
                    Some(Box::new(DefaultPseudoState::new(PseudoStateKind::Initial)))
                } else if state_data.is_end() {
                    Some(Box::new(DefaultPseudoState::new(PseudoStateKind::End)))
                } else if let Some(kind) = state_data.pseudo_state_kind() {
                    match kind {
                        PseudoStateKind::HistoryShallow | PseudoStateKind::HistoryDeep => {
                            // Handle history states separately or add to holder_list
                            continue; // Skip for now, handle later if needed
                        }
                        PseudoStateKind::Join
                        | PseudoStateKind::Fork
                        | PseudoStateKind::Choice
                        | PseudoStateKind::Junction
                        | PseudoStateKind::Entry
                        | PseudoStateKind::Exit => {
                            // Handle other pseudo-states separately or add to holder_list
                            continue; // Skip for now, handle later if needed
                        }
                        _ => Some(Box::new(DefaultPseudoState::new(kind))),
                    }
                } else {
                    None
                };

                state = self.build_state_internal(
                    state_data.state().clone(),
                    state_data.deferred().clone(),
                    state_data.entry_actions().clone(),
                    state_data.exit_actions().clone(),
                    state_data.state_actions().clone(),
                    pseudo_state,
                    state_machine_model,
                )?;
            }

            if state_data.is_initial() {
                initial_state = Some(state.box_clone()); // Assuming box_clone
                initial_action = state_data.initial_action().cloned(); // Assuming initial_action returns Option<Box<dyn Action>>
            }
            states.push(state.box_clone()); // Assuming box_clone
            state_map.insert(state_data.state().clone(), state);
        }

        // 2. Create Transition objects from TransitionData
        let mut transitions = Vec::<Arc<dyn StateMachineTransition<S, E>>>::new();
        for transition_data in transitions_data {
            let source_state = state_map.get(transition_data.source()).ok_or_else(|| {
                format!(
                    "Source state {:?} not found for transition.",
                    transition_data.source()
                )
            })?;
            let target_state = state_map.get(transition_data.target()).ok_or_else(|| {
                format!(
                    "Target state {:?} not found for transition.",
                    transition_data.target()
                )
            })?;

            let trigger: Option<Box<dyn Trigger<S, E>>> =
                if let Some(event) = transition_data.event() {
                    Some(Box::new(EventTrigger::new(event.clone())))
                } else if let Some(period) = transition_data.period() {
                    // Handle timer trigger
                    let count = transition_data.count().unwrap_or(0);
                    let mut timer_trigger = TimerTrigger::new(period, count);
                    if let Some(bf) = bean_factory {
                        timer_trigger.set_bean_factory(bf);
                    }
                    Some(Box::new(timer_trigger))
                } else {
                    None
                };

            let transition: Box<dyn StateMachineTransition<S, E>> = match transition_data.kind() {
                TransitionKind::External => {
                    Box::new(DefaultExternalTransition::new(
                        source_state.box_clone(), // Assuming box_clone
                        target_state.box_clone(), // Assuming box_clone
                        transition_data.actions().clone(),
                        transition_data.event().cloned(),
                        transition_data.guard().cloned(),
                        trigger,
                        transition_data.security_rule().cloned(),
                        transition_data.name().cloned(),
                    ))
                }
                TransitionKind::Local => {
                    Box::new(DefaultLocalTransition::new(
                        source_state.clone(), // Assuming box_clone
                        target_state.clone(), // Assuming box_clone
                        transition_data.actions().clone(),
                        transition_data.event().cloned(),
                        transition_data.guard().cloned(),
                        trigger,
                        transition_data.security_rule().cloned(),
                        transition_data.name().cloned(),
                    ))
                }
                TransitionKind::Internal => {
                    Box::new(DefaultInternalTransition::new(
                        source_state.box_clone(), // Assuming box_clone
                        transition_data.actions().clone(),
                        transition_data.event().cloned(),
                        transition_data.guard().cloned(),
                        trigger,
                        transition_data.security_rule().cloned(),
                        transition_data.name().cloned(),
                    ))
                } // Add other kinds if necessary
            };
            transitions.push(transition);
        }

        // 3. Create Initial Transition
        let initial_transition = InitialTransition::new(
            initial_state
                .as_ref()
                .ok_or("No initial state found for machine")?
                .box_clone(), // Assuming box_clone
            initial_action,
        );

        // 4. Build the final StateMachine object
        let machine = self.build_state_machine_internal(
            states,
            transitions,
            initial_state.unwrap(), // Unwrap is safe due to check above
            initial_transition,
            // initial_event: Option<Message<E>>,
            default_extended_state.box_clone(), // Assuming box_clone for ExtendedState
            // history_state: Option<PseudoState<S, E>>,
            // context_events_enabled: Option<bool>,
            // bean_name: Option<&str>, // Could come from factory or config
            machine_id.unwrap_or(state_machine_model.get_configuration_data().machine_id()), // Use provided ID or model's default
            uuid,
            state_machine_model,
        )?;

        Ok(machine)
    }

    // --- Abstract Methods (Stubs) ---
    // These methods must be implemented by concrete subclasses in Rust.

    /// Builds the internal StateMachine object.
    fn build_state_machine_internal(
        &self,
        states: Vec<Box<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        initial_transition: Arc<dyn StateMachineTransition<S, E>>,
        // initial_event: Option<Message<E>>,
        extended_state: Box<dyn ExtendedState>,
        // history_state: Option<PseudoState<S, E>>,
        // context_events_enabled: Option<bool>,
        // bean_name: Option<&str>,
        machine_id: &str,
        uuid: Option<uuid::Uuid>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Box<dyn StateMachine<S, E>>, Box<dyn std::error::Error + Send + Sync>> {
        // Use your specific error type
        // Concrete implementation required by subclass
        unimplemented!("build_state_machine_internal must be implemented by a concrete factory")
    }

    /// Builds a single State object.
    fn build_state_internal(
        &self,
        id: S,
        deferred: Vec<E>, // Assuming deferred events are a collection of E
        entry_actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
        exit_actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
        state_actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
        pseudo_state: Option<Box<dyn PseudoState<S, E>>>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Box<dyn StateMachineState<S, E>>, Box<dyn std::error::Error + Send + Sync>> {
        // Use your specific error type
        // Concrete implementation required by subclass
        unimplemented!("build_state_internal must be implemented by a concrete factory")
    }

    /// Builds a RegionState object.
    fn build_region_state_internal(
        &self,
        id: S,
        regions: Vec<Box<dyn Region<S, E>>>,
        // deferred: Option<Vec<E>>,
        // entry_actions: Option<Vec<Box<dyn Action<S, E>>>>,
        // exit_actions: Option<Vec<Box<dyn Action<S, E>>>>,
        // pseudo_state: Box<dyn PseudoState<S, E>>,
        state_machine_model: &dyn StateMachineModel<S, E>,
    ) -> Result<Box<dyn StateMachineState<S, E>>, Box<dyn std::error::Error + Send + Sync>> {
        // Use your specific error type
        // Concrete implementation required by subclass
        unimplemented!("build_region_state_internal must be implemented by a concrete factory")
    }
}

// --- Helper Functions (Corresponding to private static methods in Java) ---

/// Pops states from the stack that share the same parent.
fn pop_same_parents<S, E>(stack: &mut Vec<StateData<S, E>>) -> Vec<StateData<S, E>>
where
    S: PartialEq,
{
    let mut data = Vec::new();
    let parent = if let Some(top) = stack.last() {
        top.parent().clone()
    } else {
        return data; // Stack is empty
    };

    while let Some(current) = stack.last() {
        if current.parent() == parent {
            data.push(stack.pop().unwrap()); // Safe to unwrap as we checked !is_empty()
        } else {
            break;
        }
    }
    data.reverse(); // Reverse to get the order back as it was popped (original order)
    data
}

/// Counts the number of initial states in a collection.
fn get_initial_count<S, E>(state_datas: &[StateData<S, E>]) -> usize {
    state_datas.iter().filter(|sd| sd.is_initial()).count()
}

/// Splits a collection of StateData into groups based on their region.
fn split_into_regions<S, E>(state_datas: &[StateData<S, E>]) -> Vec<Vec<StateData<S, E>>>
where
    S: Clone,
    S: Eq + Hash,
{
    use std::collections::HashMap;
    let mut map: HashMap<Option<S>, Vec<StateData<S, E>>> = HashMap::new(); // Assuming region is S or None

    for state_data in state_datas {
        let region_key = state_data.region(); // Assuming region() returns Option<S>
        map.entry(region_key)
            .or_insert_with(Vec::new)
            .push(state_data.clone());
    }

    map.into_values().collect()
}

impl<S, E> StateMachineFactory<S, E> for BaseStateMachineFactory<S, E>
where
    S: Send + Sync + 'static,
    S: Debug,
    E: Send + Sync + 'static,
    S: Clone,
    E: Clone,
{
    fn get_state_machine(&self) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        self._get_state_machine(None, None)
    }

    /// Build a new StateMachine instance with a given machine id.
    fn get_state_machine_with_id(
        &self,
        machine_id: String,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        self._get_state_machine(None, Some(machine_id))
    }

    fn get_state_machine_with_uuid(
        &self,
        machine_id: Uuid,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        self._get_state_machine(Some(machine_id), None)
    }
}
