/// Enumerations for possible state do action policies. IMMEDIATE_CANCEL is a default setting.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum StateDoActionPolicy {
    /// Policy interrupting action immediately when state is exited.
    ImmediateCancel,

    /// Policy interrupting action after a timeout before state is exited.
    TimeoutCancel,
}
