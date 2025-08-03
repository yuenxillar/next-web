#[cfg(test)]
mod web_dev_tests {
    use std::marker::PhantomData;

    use async_trait::async_trait;
    use next_web_core::utils::any_map::AnyValue;

    use crate::state_machine::{
        config::{
            state_machine_configure::StateMachineConfigure,
            state_machine_state_configure::StateMachineStateConfigure,
            state_machine_transition_configure::StateMachineTransitionConfigure,
        },
        state_machine_context::StateContext,
        EventMessage, State, StateMachine, StateMachineAction, StateMachineListener, TestEvent,
        TestState, Transition,
    };

    #[derive(Clone, PartialEq, Eq)]
    struct TestAction;

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
                .state_machine
                .send_event(EventMessage {
                    event: TestEvent::Close,
                    payload: Some(AnyValue::Null),
                })
                .await;
        }
    }

    #[derive(Clone, PartialEq, Eq)]
    struct TestAction1 {
        var: String,
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

    #[test]
    fn test_hash() {
        let mut map = hashbrown::HashMap::new();
        map.insert(
            (
                String::new(),
                TestState::Ready,
                TestState::Run,
                TestEvent::Open,
            ),
            0,
        );
        map.insert(
            (
                String::new(),
                TestState::Run,
                TestState::Ready,
                TestEvent::Close,
            ),
            1,
        );

        map.get(&(
            String::new(),
            TestState::Run,
            TestState::Ready,
            TestEvent::Close,
        ))
        .map(|s| println!("i32: {}", s));
    }

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
        S: Send + Sync + Clone,
        E: Send + Sync + Clone,
    {
        async fn state_changed(&self, from: S, to: S, event: E) {}

        async fn event_not_accepted(&self, event: EventMessage<E>) {}
    }

    #[tokio::test]
    async fn test_state_machine() {
        let configure = StateMachineConfigure::<TestState, TestEvent>::default();

        let state_configure = StateMachineStateConfigure::default();

        let action = TestAction;
        let action1 = TestAction1 {
            var: String::from("hello"),
        };

        let transition_configure = StateMachineTransitionConfigure::default()
            .with(
                TestState::Ready,
                TestState::Run,
                TestEvent::Open,
                Box::new(action),
            )
            .with(
                TestState::Run,
                TestState::Ready,
                TestEvent::Close,
                Box::new(action1),
            );

        let state_machie =
            StateMachine::from_configure(configure, state_configure, transition_configure)
                .set_id("testStateMachine")
                .add_state_listener(TestEventListener::default());

        let ac_self = state_machie.start();
        ac_self
            .send_event(EventMessage {
                event: TestEvent::Open,
                payload: Some(AnyValue::Boolean(true)),
            })
            .await;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
