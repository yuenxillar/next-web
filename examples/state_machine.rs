use std::sync::Arc;

use next_web::core::{async_trait, messaging::GenericMessage, messaging::Message};
use next_web_core::{any::AnyValue, error::BoxError};
use next_web_state_machine::{
    StateContext, StateMachine,
    autoconfigure::StateMachineAutoConfiguration,
    config::{
        action::StateMachineAction,
        builders::{
            StateMachineConfigBuilder, StateMachineConfigurationBuilder,
            StateMachineConfigurationConfigurer, StateMachineConfigurer, StateMachineModelBuilder,
            StateMachineStateBuilder, StateMachineStateConfigurer, StateMachineTransitionBuilder,
            StateMachineTransitionConfigurer,
        },
        common::Configurer,
        state_machine_config::StateMachineConfig,
    },
    listener::StateMachineListener,
    state::StateMachineState,
    transition::StateMachineTransition,
};

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
        config: &mut StateMachineConfigurationBuilder<OrderState, OrderEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        struct DefaultListener;

        impl StateMachineListener<OrderState, OrderEvent> for DefaultListener {
            fn state_changed(
                &self,
                _from: &dyn StateMachineState<OrderState, OrderEvent>,
                _to: &dyn StateMachineState<OrderState, OrderEvent>,
            ) {
            }

            fn state_entered(&self, _state: &dyn StateMachineState<OrderState, OrderEvent>) {}

            fn state_exited(&self, _state: &dyn StateMachineState<OrderState, OrderEvent>) {}

            fn event_not_accepted(&self, _event: &dyn Message<OrderEvent>) {}

            fn transition(&self, transition: &dyn StateMachineTransition<OrderState, OrderEvent>) {
                println!("kind: {:?}", transition.kind())
            }

            fn transition_started(
                &self,
                _transition: &dyn StateMachineTransition<OrderState, OrderEvent>,
            ) {
            }

            fn transition_ended(
                &self,
                _transition: &dyn StateMachineTransition<OrderState, OrderEvent>,
            ) {
            }

            fn state_machine_started(
                &self,
                _state_machine: &dyn StateMachine<OrderState, OrderEvent>,
            ) {
            }

            fn state_machine_stopped(
                &self,
                _state_machine: &dyn StateMachine<OrderState, OrderEvent>,
            ) {
            }

            fn state_machine_error(
                &self,
                _state_machine: &dyn StateMachine<OrderState, OrderEvent>,
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
        _model: &mut StateMachineModelBuilder<OrderState, OrderEvent>,
    ) -> Result<(), next_web_core::error::BoxError> {
        Ok(())
    }

    fn state_configure(
        &mut self,
        states: &mut StateMachineStateBuilder<OrderState, OrderEvent>,
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
        transitions: &mut StateMachineTransitionBuilder<OrderState, OrderEvent>,
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
    let config = OrderStateMachineConfig {};
    let state_machine = StateMachineAutoConfiguration::build(config)?;
    state_machine.start().await?;

    state_machine
        .send_event(Arc::new(GenericMessage::with_payload(OrderEvent::Pay)))
        .await?;

    Ok(())
}
