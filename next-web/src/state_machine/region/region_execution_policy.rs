/// Enumerations for possible region execution policies.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum RegionExecutionPolicy {
    /// Policy executing regions sequentially.
    Sequential,

    /// Policy executing regions parallelly.
    Parallel,
}
