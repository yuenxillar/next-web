pub struct StateMachineSystemConstants;

impl StateMachineSystemConstants {
    /// Default bean id for state machine.
    pub const DEFAULT_ID_STATEMACHINE: &str = "stateMachine";

    /// Default bean id for state machine factory.
    pub const DEFAULT_ID_STATEMACHINEFACTORY: &str = "stateMachineFactory";

    /// Default bean id for state machine event publisher.
    pub const DEFAULT_ID_EVENT_PUBLISHER: &str = "stateMachineEventPublisher";

    /// State machine id key for headers and variables
    pub const STATEMACHINE_IDENTIFIER: &str = "_sm_id_";

    /// Constant storing errors in a reactor context
    pub const REACTOR_CONTEXT_ERRORS: &str = "stateMachineErrors";
}
