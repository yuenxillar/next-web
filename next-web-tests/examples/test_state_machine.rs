use std::sync::Arc;

use next_web::{
    application::Application,
    core::{
        ApplicationContext, async_trait, context::properties::ApplicationProperties,
        messaging::generic_message::GenericMessage,
        traits::message::Message,
    },
};
use next_web_core::{anys::any_value::AnyValue, error::BoxError};
use next_web_state_machine::{
    autoconfigure::state_machine_auto_configuration::StateMachineAutoConfiguration,
    config::{
        action::StateMachineAction,
        builders::{
            state_machine_config_builder::StateMachineConfigBuilder,
            state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
            state_machine_configurer::StateMachineConfigurer,
            state_machine_state_configurer::StateMachineStateConfigurer,
            state_machine_transition_configurer::StateMachineTransitionConfigurer,
        },
        common::configurer::Configurer,
        state_machine_config::StateMachineConfig,
    },
    listener::state_machine_listener::StateMachineListener,
    state_context::StateContext,
};

#[derive(Clone, Default)]
#[allow(unused)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    async fn on_ready(
        &self,
        _ctx: &mut ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum OrderState {
    #[default]
    Crateed,
    Paid,
    Shipped,
    Delivered,
    Completed,
    Cancelled,
    Refunded,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OrderEvent {
    Pay,
    Ship,
    Deliver,
    Complete,
    Cancel,
    Refund,
}

#[derive(Clone)]
struct OrderStateMachineConfig;

impl StateMachineConfigurer<OrderState, OrderEvent> for OrderStateMachineConfig {
    fn config_configure(
        &mut self,
        config: &mut next_web_state_machine::config::builders::state_machine_configuration_builder::StateMachineConfigurationBuilder<OrderState, OrderEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        struct DefaultListener;

        impl StateMachineListener<OrderState, OrderEvent> for DefaultListener {
            fn state_changed(
                &self,
                _from: &dyn next_web_state_machine::state::StateMachineState<OrderState, OrderEvent>,
                _to: &dyn next_web_state_machine::state::StateMachineState<OrderState, OrderEvent>,
            ) {
            }

            fn state_entered(
                &self,
                _state: &dyn next_web_state_machine::state::StateMachineState<OrderState, OrderEvent>,
            ) {
            }

            fn state_exited(
                &self,
                _state: &dyn next_web_state_machine::state::StateMachineState<OrderState, OrderEvent>,
            ) {
            }

            fn event_not_accepted(&self, _event: &dyn Message<OrderEvent>) {}

            fn transition(
                &self,
                transition: &dyn next_web_state_machine::transition::StateMachineTransition<
                    OrderState,
                    OrderEvent,
                >,
            ) {
                println!("kind: {:?}", transition.kind())
            }

            fn transition_started(
                &self,
                _transition: &dyn next_web_state_machine::transition::StateMachineTransition<
                    OrderState,
                    OrderEvent,
                >,
            ) {
            }

            fn transition_ended(
                &self,
                _transition: &dyn next_web_state_machine::transition::StateMachineTransition<
                    OrderState,
                    OrderEvent,
                >,
            ) {
            }

            fn state_machine_started(
                &self,
                _state_machine: &dyn next_web_state_machine::StateMachine<OrderState, OrderEvent>,
            ) {
            }

            fn state_machine_stopped(
                &self,
                _state_machine: &dyn next_web_state_machine::StateMachine<OrderState, OrderEvent>,
            ) {
            }

            fn state_machine_error(
                &self,
                _state_machine: &dyn next_web_state_machine::StateMachine<OrderState, OrderEvent>,
                _error: &dyn std::error::Error,
            ) {
            }

            fn extended_state_changed(&self, _key: &str, _value: &AnyValue) {}

            fn state_context(&self, _state_context: &dyn StateContext<OrderState, OrderEvent>) {}
        }
        config
            .with_configuration()?
            .machine_id(String::from("orderStateMachine"))
            .listener(Arc::new(DefaultListener));

        Ok(())
    }

    fn model_configure(
        &mut self,
        model: &mut next_web_state_machine::config::builders::state_machine_model_builder::StateMachineModelBuilder<OrderState, OrderEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        Ok(())
    }

    fn state_configure(
        &mut self,
        states: &mut next_web_state_machine::config::builders::state_machine_state_builder::StateMachineStateBuilder<OrderState, OrderEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        states
            .with_states()?
            .initial(OrderState::Crateed)
            .states(vec![
                OrderState::Crateed,
                OrderState::Paid,
                OrderState::Shipped,
                OrderState::Delivered,
                OrderState::Completed,
                OrderState::Cancelled,
                OrderState::Refunded,
            ])
            .end(OrderState::Completed)
            .end(OrderState::Cancelled)
            .end(OrderState::Refunded);

        Ok(())
    }

    fn transition_configure(
        &mut self,
        transitions: &mut next_web_state_machine::config::builders::state_machine_transition_builder::StateMachineTransitionBuilder<OrderState, OrderEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        #[derive(Clone)]
        struct PayAction;

        #[async_trait]
        impl StateMachineAction<OrderState, OrderEvent> for PayAction {
            async fn execute(
                &self,
                _context: &dyn StateContext<OrderState, OrderEvent>,
            ) -> Result<(), BoxError> {
                println!("hello!!!");
                Ok(())
            }
        }

        #[derive(Clone)]
        struct CompleteAction;

        #[async_trait]
        impl StateMachineAction<OrderState, OrderEvent> for CompleteAction {
            async fn execute(
                &self,
                _context: &dyn StateContext<OrderState, OrderEvent>,
            ) -> Result<(), BoxError> {
                Ok(())
            }
        }

        transitions
            .with_external()?
            .source(OrderState::Crateed)
            .target(OrderState::Paid)
            .event(OrderEvent::Pay)
            .action(Arc::new(PayAction {}))
            .and()
            .with_external()?
            .source(OrderState::Delivered)
            .target(OrderState::Completed)
            .event(OrderEvent::Complete)
            .action(Arc::new(CompleteAction {}));

        Ok(())
    }
}

impl
    Configurer<
        StateMachineConfig<OrderState, OrderEvent>,
        StateMachineConfigBuilder<OrderState, OrderEvent>,
    > for OrderStateMachineConfig
{
    fn init(
        &mut self,
        _builder: &StateMachineConfigBuilder<OrderState, OrderEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        Ok(())
    }

    fn configure(
        &mut self,
        _builder: &StateMachineConfigBuilder<OrderState, OrderEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        Ok(())
    }

    fn is_assignable(&self, _builder: &StateMachineConfigBuilder<OrderState, OrderEvent>) -> bool {
        true
    }
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // TestApplication::run().await;

    let config = OrderStateMachineConfig {};
    let state_machine = StateMachineAutoConfiguration::build(config)?;
    state_machine.start().await?;

    state_machine
        .send_event(Box::new(GenericMessage::with_payload(OrderEvent::Pay)))
        .await?;

    Ok(())
}
