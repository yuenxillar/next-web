use std::{fmt::Debug, marker::PhantomData};

use next_web_core::{
    async_trait, context::properties::ApplicationProperties, anys::any_value::AnyValue,
    ApplicationContext,
};
use next_web_dev::{
    application::Application,
    state_machine::{
        config::state_machine_transition_configure::StateMachineTransitionConfigure,
        state_machine_context::StateContext, state_machine_generator::StateMachineGenerator,
        EventMessage, StateMachineAction, StateMachineListener, Transition,
    },
};

#[derive(Clone)]
struct TestEventListener<S, E>(PhantomData<S>, PhantomData<E>);

impl<S, E> Default for TestEventListener<S, E> {
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}

#[async_trait]
impl<S, E> StateMachineListener<S, E> for TestEventListener<S, E>
where
    S: Send + Sync,
    S: Debug + Clone,
    E: Send + Sync,
    E: Debug + Clone,
{
    async fn state_changed(&self, from: S, to: S, event: E) {
        println!(
            "state changed from [{:?}] -> [{:?}] with event: [{:?}]",
            from, to, event
        )
    }

    async fn event_not_accepted(&self, event: EventMessage<E>) {
        println!("event not accepted: {:?}", event)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TestAction;

#[async_trait]
impl StateMachineAction<TestState, TestEvent> for TestAction {
    fn transition(&self) -> Transition<TestState, TestEvent> {
        Transition {
            source: TestState::Ready,
            target: TestState::Run,
            event: TestEvent::Open,
        }
    }

    async fn execute(&mut self, context: StateContext<TestState, TestEvent>) {
        println!(
            "TestAction event message with payload: {:?}",
            context.payload().map(|s| s.as_boolean())
        );
        context
            .state_machine()
            .send_event(EventMessage::new(TestEvent::Close, Some(AnyValue::Null)))
            .await;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TestAction1(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum TestState {
    #[default]
    Ready,
    Run,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TestEvent {
    Open,
    Close,
}

#[async_trait]
impl StateMachineAction<TestState, TestEvent> for TestAction1 {
    fn transition(&self) -> Transition<TestState, TestEvent> {
        Transition {
            source: TestState::Run,
            target: TestState::Ready,
            event: TestEvent::Close,
        }
    }

    async fn execute(&mut self, context: StateContext<TestState, TestEvent>) {
        println!(
            "TestAction1 event message with payload: {:?}",
            context.payload()
        );
    }
}

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().route("/", axum::routing::get(|| async move { "Ok" }))
    }

    async fn before_start(&self, _ctx: &mut ApplicationContext) {
        tokio::spawn(async move {
            let transition_configure = StateMachineTransitionConfigure::default()
                .with(
                    TestState::Ready,
                    TestState::Run,
                    TestEvent::Open,
                    Box::new(TestAction),
                )
                .with(
                    TestState::Run,
                    TestState::Ready,
                    TestEvent::Close,
                    Box::new(TestAction1(String::from("dev"))),
                );
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;

            let state_machie =
                StateMachineGenerator::generate("testStateMachine", transition_configure)
                    .add_state_listener(TestEventListener::default());

            let machine = state_machie.start().await;
            machine.send_event(EventMessage::new(
                TestEvent::Open,
                Some(AnyValue::Boolean(true)),
            ))
            .await;
        });
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}
