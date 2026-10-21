//! Tests of the configuration which reaches the runtime.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use next_web_core::error::BoxError;
use next_web_state_machine::BoxedStateAction;
use next_web_state_machine::action::state_do_action_policy::StateDoActionPolicy;
use next_web_state_machine::config::base_state_machine_factory::BaseStateMachineFactory;
use next_web_state_machine::config::model::configuration_data::ConfigurationData;
use next_web_state_machine::config::model::default_state_machine_model::DefaultStateMachineModel;
use next_web_state_machine::config::model::state_data::StateData;
use next_web_state_machine::config::model::transition_data::TransitionData;
use next_web_state_machine::config::model::transitions_data::TransitionsData;
use next_web_state_machine::config::state_machine_factory::StateMachineFactory;
use next_web_state_machine::region::region_execution_policy::RegionExecutionPolicy;
use next_web_state_machine::state_context::StateContext;

/// A configuration with the default values except the given policy.
fn configuration(
    policy: Option<StateDoActionPolicy>,
    timeout: Option<Duration>,
    region_policy: Option<RegionExecutionPolicy>,
) -> ConfigurationData<i32, i32> {
    ConfigurationData::with_all_parameters(
        false,
        None,
        Vec::new(),
        false,
        None,
        None,
        None,
        None,
        false,
        None,
        Some("policy".to_owned()),
        None,
        Vec::new(),
        None,
        policy,
        timeout,
        region_policy,
    )
}

/// A state action which reports whether it completed.
fn blocking_action(completed: Arc<AtomicBool>) -> BoxedStateAction<i32, i32> {
    Arc::new(move |_context: &dyn StateContext<i32, i32>| {
        let completed = completed.clone();

        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            completed.store(true, Ordering::SeqCst);

            Ok(()) as Result<(), BoxError>
        })
    })
}

/// Builds a machine with one state whose state action blocks.
fn build_machine(
    configuration: ConfigurationData<i32, i32>,
    completed: Arc<AtomicBool>,
) -> Result<Arc<dyn next_web_state_machine::StateMachine<i32, i32>>, BoxError> {
    let mut initial = StateData::<i32, i32>::with_initial(1, true);
    initial.set_state_actions(vec![blocking_action(completed)]);

    let model = DefaultStateMachineModel::new(
        configuration,
        Some(vec![initial, StateData::<i32, i32>::new(2)].into()),
        Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
    );

    BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None).get_state_machine()
}

#[tokio::test]
async fn the_timeout_policy_of_the_configuration_reaches_the_state() {
    let completed = Arc::new(AtomicBool::new(false));

    let machine = build_machine(
        configuration(
            Some(StateDoActionPolicy::TimeoutCancel),
            Some(Duration::from_millis(20)),
            None,
        ),
        completed.clone(),
    )
    .expect("the machine builds");

    let started = std::time::Instant::now();
    machine.start().await.expect("the machine starts");

    assert!(
        started.elapsed() < Duration::from_millis(150),
        "the state action is bounded by the configured timeout"
    );
    assert!(
        !completed.load(Ordering::SeqCst),
        "the state action was cancelled by its timeout"
    );
    assert_eq!(machine.state().map(|state| *state.id()), Some(1));
}

#[tokio::test]
async fn a_default_policy_waits_for_the_state_action() {
    let completed = Arc::new(AtomicBool::new(false));

    let machine = build_machine(configuration(None, None, None), completed.clone())
        .expect("the machine builds");

    machine.start().await.expect("the machine starts");

    assert!(
        completed.load(Ordering::SeqCst),
        "the state action runs inline without a timeout"
    );
}

#[tokio::test]
async fn a_parallel_region_policy_is_rejected() {
    let completed = Arc::new(AtomicBool::new(false));

    let result = build_machine(
        configuration(None, None, Some(RegionExecutionPolicy::Parallel)),
        completed,
    );

    let error = match result {
        Ok(_) => panic!("the parallel region policy is not supported"),
        Err(error) => error,
    };

    assert!(
        error.to_string().contains("parallel region execution"),
        "the error names the unsupported policy, got: {error}"
    );
}

#[tokio::test]
async fn a_sequential_region_policy_is_accepted() {
    let completed = Arc::new(AtomicBool::new(false));

    let machine = build_machine(
        configuration(None, None, Some(RegionExecutionPolicy::Sequential)),
        completed,
    )
    .expect("the sequential region policy matches the runtime");

    machine.start().await.expect("the machine starts");
    assert_eq!(machine.state().map(|state| *state.id()), Some(1));
}
