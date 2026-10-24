use crate::ApplicationContext;

/// A single condition that must be matched in order for a component to be
/// registered.
///
/// Conditions are evaluated immediately before a component definition is due
/// to be registered, and are free to veto registration based on any criteria
/// that can be determined at that point.
///
/// Conditions must not interact with live component instances. For finer
/// control over conditions that need to interact with configuration
/// components, consider extending the evaluation with a dedicated
/// configuration-aware condition type.
///
/// Multiple conditions applied to the same type or the same method are
/// evaluated in a deterministic order derived from their relative priority.
pub trait Condition {
    /// Determines whether the condition matches.
    ///
    /// # Parameters
    /// - `context`: the context providing access to the environment,
    ///   component registry, and resource loader.
    ///
    /// # Returns
    /// `true` if the condition matches and the component may be registered,
    /// or `false` to veto registration of the annotated component.
    fn matches(&self, context: &dyn ApplicationContext) -> bool;
}
