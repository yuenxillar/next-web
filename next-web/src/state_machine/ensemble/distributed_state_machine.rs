// use futures::stream::StreamExt;
// use next_web_core::async_trait;
// use next_web_core::error::BoxError;
// use next_web_core::messaging::support::message_builder::MessageBuilder;
// use next_web_core::traits::message::Message;
// use std::sync::Arc;
// use std::{fmt, marker::PhantomData};
// use tracing::{debug, error, info, trace};

// use crate::state_machine::ensemble::ensemble_listener::EnsembleListener;
// use crate::state_machine::ensemble::state_machine_ensemble_error::StateMachineEnsembleError;
// use crate::state_machine::listener::state_machine_listener::StateMachineListener;
// use crate::state_machine::region::Region;
// use crate::state_machine::state::StateMachineState;
// use crate::state_machine::state_context::StateContext;
// use crate::state_machine::state_machine_context::StateMachineContext;
// use crate::state_machine::state_machine_event_result::StateMachineEventResult;
// use crate::state_machine::state_machine_system_constants::StateMachineSystemConstants;
// use crate::state_machine::support::lifecycle_object_support::{
//     LifecycleObjectSupport, LifecycleObjectSupportExt,
// };
// use crate::state_machine::support::state_machine_interceptor::StateMachineInterceptor;
// use crate::state_machine::transition::transition_kind::TransitionKind;
// use crate::state_machine::transition::StateMachineTransition;
// use crate::state_machine::{ensemble::state_machine_ensemble::StateMachineEnsemble, StateMachine};

// /// `DistributedStateMachine` is wrapping a real `StateMachine` and works
// /// together with a `StateMachineEnsemble` order to provide a distributed state
// /// machine.
// ///
// /// Every distributed state machine will enter its initial state regardless of
// /// a distributed state status.
// #[derive(Clone)]
// pub struct DistributedStateMachine<S, E, T1, T2> {
//     ensemble: Arc<dyn StateMachineEnsemble<S, E>>,
//     delegate: Arc<dyn StateMachine<S, E>>,
//     listener: LocalEnsembleListener<S, E>,
//     interceptor: LocalStateMachineInterceptor<S, E>,

//     support: LifecycleObjectSupport,
// }

// impl<S, E> DistributedStateMachine<S, E> {
//     /// Instantiates a new distributed state machine.
//     pub fn new(
//         ensemble: Arc<dyn StateMachineEnsemble<S, E>>,
//         delegate: Arc<dyn StateMachine<S, E>>,
//     ) -> Self {
//         let interceptor = LocalStateMachineInterceptor::default();
//         let listener = LocalEnsembleListener::default();

//         DistributedStateMachine {
//             ensemble,
//             delegate,
//             listener,
//             interceptor,
//             support: Default::default(),
//         }
//     }

//     fn add_machine_identifier(&self) -> impl Fn(Box<dyn Message<E>>) -> Box<dyn Message<E>> {
//         let delegate_uuid = self.delegate.get_uuid();
//         move |msg| {
//             MessageBuilder::from_message(msg)
//                 .header(
//                     StateMachineSystemConstants::STATEMACHINE_IDENTIFIER.to_string(),
//                     delegate_uuid,
//                 )
//                 .build()
//         }
//     }
// }

// #[async_trait]
// impl<S, E> LifecycleObjectSupportExt for DistributedStateMachine<S, E> {
//     fn on_init(&mut self) -> Result<(), BoxError> {
//         // TODO: should we register with all, not just top one?
//         self.delegate
//             .get_state_machine_accessor()
//             .do_with_region(Box::new(|accessor| {
//                 accessor.add_state_machine_interceptor(self.interceptor.clone());
//             }));
//         Ok(())
//     }

//     async fn do_pre_start_reactively(&self) {
//         self.ensemble
//             .add_ensemble_listener(Arc::new(self.clone()))
//             .await;
//         // self.ensemble.join().await;
//     }

//     async fn do_pre_stop_reactively(&self) {
//         self.ensemble.remove_ensemble_listener(&self.listener);
//         // self.ensemble.leave(self).await;
//     }
// }

// impl<S, E> Region<S, E> for DistributedStateMachine<S, E> {
//     async fn send_event(
//         &self,
//         event: Box<dyn Message<E>>,
//     ) -> Result<Box<dyn StateMachineEventResult<S, E>>, BoxError> {
//         self.delegate
//             .send_event_message(self.add_machine_identifier()(event))
//             .await
//     }

//     async fn send_events(
//         &self,
//         events: Vec<Box<dyn Message<E>>>,
//     ) -> Result<Box<dyn StateMachineEventResult<S, E>>, BoxError> {
//         let mapped_events = event.map(|e| self.add_machine_identifier()(e));
//         self.delegate.send_events(mapped_events).await
//     }

//     fn state(&self) -> &dyn StateMachineState<S, E> {
//         self.delegate.get_state()
//     }

//     fn states(&self) -> Vec<&dyn StateMachineState<S, E>> {
//         self.delegate.get_states()
//     }

//     fn transitions(&self) -> Vec<Arc<dyn StateMachineTransition<S, E>>> {
//         self.delegate.get_transitions()
//     }

//     fn is_complete(&self) -> bool {
//         self.delegate.is_complete()
//     }

//     fn add_state_listener(
//         &self,
//         listener: Arc<dyn StateMachineListener<S, E>>,
//     ) -> Result<(), BoxError> {
//         self.delegate.add_state_listener(listener)
//     }

//     fn remove_state_listener(
//         &self,
//         listener: &dyn StateMachineListener<S, E>,
//     ) -> Result<(), BoxError> {
//         self.delegate.remove_state_listener(listener)
//     }

//     fn id(&self) -> String {
//         self.delegate.id()
//     }
// }

// impl<S, E> fmt::Display for DistributedStateMachine<S, E> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "DistributedStateMachine [delegate={}]", self.delegate)
//     }
// }

// /// We intercept state changes order to attempt to update global
// /// distributed state. This attempt is sent to an ensemble which will
// /// tell us if that attempt was successful.
// #[derive(Clone)]
// struct LocalStateMachineInterceptor<S, E>(PhantomData<(S, E)>);

// impl<S, E> LocalStateMachineInterceptor<S, E> {}

// impl<S, E> Default for LocalStateMachineInterceptor<S, E> {
//     fn default() -> Self {
//         Self(PhantomData)
//     }
// }

// impl<S, E> StateMachineInterceptor<S, E> for LocalStateMachineInterceptor<S, E>
// where
//     E: Send + Sync,
// {
//     fn pre_event<'a>(
//         &self,
//         message: &'a mut dyn Message<E>,
//         state_machine: &dyn StateMachine<S, E>,
//     ) -> Result<&'a mut dyn Message<E>, BoxError> {
//         todo!()
//     }

//     fn pre_state_change(
//         &self,
//         state: &dyn StateMachineState<S, E>,
//         message: &dyn Message<E>,
//         transition: &dyn StateMachineTransition<S, E>,
//         state_machine: &dyn StateMachine<S, E>,
//         root_state_machine: &dyn StateMachine<S, E>,
//     ) -> Result<(), BoxError> {
//         trace!(
//             "Received pre_state_change from {:?} for delegate {:?}",
//             state_machine,
//             self.delegate
//         );

//         // only handle if state change originates from this dist machine
//         if let Some(header_uuid) = message
//             .get_headers()
//             .get(&StateMachineSystemConstants::STATEMACHINE_IDENTIFIER)
//         {
//             if header_uuid == &self.delegate.get_uuid() {
//                 if let Some(t) = transition {
//                     let context = DefaultStateMachineContext::new(
//                         t.get_target().get_id(),
//                         msg.get_payload(),
//                         msg.get_headers(),
//                         state_machine.get_extended_state(),
//                     );
//                     self.ensemble.set_state(context).await;
//                 }
//             }
//         }

//         Ok(())
//     }

//     fn post_state_change(
//         &self,
//         state: &dyn StateMachineState<S, E>,
//         message: &dyn Message<E>,
//         transition: &dyn StateMachineTransition<S, E>,
//         state_machine: &dyn StateMachine<S, E>,
//         root_state_machine: &dyn StateMachine<S, E>,
//     ) {
//     }

//     fn pre_transition<'a>(
//         &self,
//         state_context: &'a dyn StateContext<S, E>,
//     ) -> Result<&'a dyn StateContext<S, E>, BoxError> {
//         todo!()
//     }

//     fn post_transition(
//         &self,
//         state_context: &dyn StateContext<S, E>,
//     ) -> Result<&dyn StateContext<S, E>, BoxError> {
//         // only handle if state change originates from this dist machine
//         if let Some(t) = state_context.transition() {
//             if t.kind() == TransitionKind::Internal {
//                 if let Some(header_uuid) = state_context
//                     .get_message_header(&StateMachineSystemConstants::STATEMACHINE_IDENTIFIER)
//                 {
//                     if header_uuid == &self.delegate.get_uuid() {
//                         let current = self.ensemble.get_state();
//                         if let Some(ctx) = current {
//                             let new_context = DefaultStateMachineContext::new(
//                                 ctx.get_state(),
//                                 state_context.get_event(),
//                                 state_context.get_message_headers(),
//                                 state_context.get_state_machine().get_extended_state(),
//                             );
//                             self.ensemble.set_state(new_context);
//                         } else if let Some(s) = state_context.get_state_machine().get_state() {
//                             // if current ensemble state is None, get it from sm itself
//                             let new_context = DefaultStateMachineContext::new(
//                                 s.get_id(),
//                                 state_context.get_event(),
//                                 state_context.get_message_headers(),
//                                 state_context.get_state_machine().get_extended_state(),
//                             );
//                             self.ensemble.set_state(new_context);
//                         }
//                     }
//                 }
//             }
//         }

//         Ok(())
//     }

//     fn state_machine_error(
//         &self,
//         state_machine: &dyn StateMachine<S, E>,
//         error: Box<dyn std::error::Error>,
//     ) -> Box<dyn std::error::Error> {
//         todo!()
//     }
// }

// /// Bridge for instructing delegating machine based on what
// /// is happening in an ensemble.
// #[derive(Clone)]
// struct LocalEnsembleListener<S, E>(PhantomData<(S, E)>);

// impl<S, E> LocalEnsembleListener<S, E> {}

// impl<S, E> Default for LocalEnsembleListener<S, E> {
//     fn default() -> Self {
//         Self(PhantomData)
//     }
// }

// #[async_trait]
// impl<S, E> EnsembleListener<S, E> for LocalEnsembleListener<S, E>
// where
//     S: Clone + Send + Sync + 'static,
//     E: Clone + Send + Sync + 'static,
// {
//     fn state_machine_joined(
//         &self,
//         state_machine: &dyn StateMachine<S, E>,
//         context: &dyn StateMachineContext<S, E>,
//     ) {
//         debug!(
//             "Event state_machine_joined state_machine={:?} context={:?}",
//             state_machine, context
//         );

//         if Arc::ptr_eq(&sm, &self.delegate) {
//             self.delegate.stop();
//             self.delegate.set_state_machine_error(None);

//             // I'm now successfully joined, so set delegating
//             // sm to current known state by a context.
//             debug!("Joining with context {:?}", ctx);

//             self.delegate
//                 .get_state_machine_accessor()
//                 .do_with_all_regions(|accessor| {
//                     accessor.reset_state_machine(ctx.clone());
//                 });

//             info!(
//                 "Requesting to start delegating state machine {:?}",
//                 self.delegate
//             );
//             info!("Delegating machine id {:?}", self.delegate.get_uuid());
//             self.delegate.start();
//         }
//     }

//     fn state_machine_left(
//         &self,
//         state_machine: &dyn StateMachine<S, E>,
//         context: &dyn StateMachineContext<S, E>,
//     ) {
//         if Arc::ptr_eq(&sm, &self.delegate) {
//             info!(
//                 "Requesting to stop delegating state machine {:?}",
//                 self.delegate
//             );
//             self.delegate.stop();
//         }
//     }

//     fn state_changed(&self, context: &dyn StateMachineContext<S, E>) {
//         // do not pass if state change was originated from this dist machine
//         if let Some(header_uuid) = context
//             .event_headers()
//             .get(&StateMachineSystemConstants::STATEMACHINE_IDENTIFIER)
//         {
//             if header_uuid != &self.delegate.get_uuid() {
//                 let msg = MessageBuilder::with_payload(context.event())
//                     .copy_headers(context.event_headers())
//                     .build();
//                 self.delegate.send_event(futures::stream::iter(vec![msg]));
//             }
//         }
//     }

//     fn ensemble_error(&self, error: &StateMachineEnsembleError) {
//         error!("Ensemble error: {:?}", error);
//         self.delegate
//             .set_state_machine_error(Some(Box::new(exception.clone())));
//         // In Rust we cannot throw exception like in Java, so we'll handle appropriately
//     }

//     fn ensemble_leader_granted(&self, state_machine: &dyn StateMachine<S, E>) {}

//     fn ensemble_leader_revoked(&self, state_machine: &dyn StateMachine<S, E>) {}
// }
