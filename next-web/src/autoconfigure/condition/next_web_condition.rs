use next_web_core::ApplicationContext;

use crate::autoconfigure::condition::ConditionOutcome;

/// Base trait for all condition implementations.
///
/// Provides a shared evaluation flow: compute the outcome, log it, record it
/// into the evaluation report, and finally return whether it matched. The
/// public entry point `matches` is intentionally not overridable; implementors
/// only supply [`NextWebConditionExt::get_match_outcome`].
///
/// This mirrors the role of the original base condition class, but is split
/// into two traits so that the evaluation flow stays sealed while the outcome
/// logic stays open for extension.
pub trait NextWebCondition {
    /// Evaluates the condition against the given context.
    ///
    /// This method is the fixed entry point. It:
    /// 1. derives a readable name for the element being checked,
    /// 2. delegates to [`NextWebConditionExt::get_match_outcome`],
    /// 3. logs the outcome at trace level,
    /// 4. records the outcome into the evaluation report,
    /// 5. returns whether the outcome matched.
    ///
    /// # Errors
    ///
    /// Returns [`ConditionError`] when the outcome computation fails. Errors
    /// raised while evaluating are wrapped with the name of the element so
    /// that diagnostics point at the right place.
    fn matches(&self, context: &dyn ApplicationContext) -> Result<bool, ConditionError>;

    /// Logs the outcome at trace level.
    ///
    /// Mirrors the original `logOutcome` method. Implementations may override
    /// this to integrate with a different logging backend.
    fn log_outcome(&self, outcome: &ConditionOutcome) {
        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(
                target: "next_web::autoconfigure::condition",
                "{}",
                self.log_message( outcome)
            );
        }
    }

    /// Builds the log message for an outcome.
    ///
    /// Format: `Condition <ShortName> on <Target> matched|did not match
    /// [due to <Message>]`.
    fn log_message(&self, outcome: &ConditionOutcome) -> String {
        let mut message = String::new();
        message.push_str("Condition ");
        message.push_str(self.short_name());
        message.push_str(" on ");
        message.push_str("matches");
        message.push_str(" ");
        message.push_str(if outcome.is_match() {
            " matched"
        } else {
            " did not match"
        });
        if let Some(detail) = outcome.message() {
            message.push_str(" due to ");
            message.push_str(&detail);
        }
        message
    }

    /// Returns the short name of this condition, used in log messages.
    ///
    /// Defaults to the fully qualified type name of `Self`. Implementations
    /// may override this to return a friendlier name.
    fn short_name(&self) -> &str {
        let type_name = std::any::type_name::<Self>();
        type_name.rsplit("::").next().unwrap_or(&type_name)
    }
}

/// Extension trait implemented by concrete conditions.
///
/// Implementors only describe how to compute an outcome. The surrounding
/// evaluation flow is provided by [`NextWebCondition`].
pub trait NextWebConditionExt {
    /// Determines the outcome of the match along with suitable log output.
    ///
    /// # Errors
    ///
    /// Returns [`ConditionError`] when the outcome cannot be computed, for
    /// example because a referenced type is missing or a rule misbehaves.
    fn get_match_outcome(
        &self,
        context: &dyn ApplicationContext,
    ) -> Result<ConditionOutcome, ConditionError>;
}

impl<T: NextWebConditionExt> NextWebCondition for T {
    fn matches(&self, context: &dyn ApplicationContext) -> Result<bool, ConditionError> {
        let outcome = self.get_match_outcome(context)?;
        self.log_outcome(&outcome);
        Ok(outcome.is_match())
    }
}

/// Errors produced while evaluating a condition.
#[derive(Debug)]
pub enum ConditionError {
    /// A type referenced by the condition could not be found.
    ///
    /// This mirrors the original `NoClassDefFoundError` handling and carries
    /// the missing type name for diagnostics.
    TypeNotFound {
        /// The name of the element being evaluated.
        target: String,
        /// The missing type name, when known.
        missing: Option<String>,
    },
    /// The condition failed during evaluation.
    Evaluation(String),
}

impl std::fmt::Display for ConditionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConditionError::TypeNotFound { target, missing } => {
                write!(f, "could not evaluate condition on {target}")?;
                if let Some(missing) = missing {
                    write!(
                        f,
                        " because {missing} was not found; make sure the configuration \
                         does not rely on that type"
                    )?;
                }
                Ok(())
            }
            ConditionError::Evaluation(message) => {
                write!(f, "The condition failed during evaluation: {message}")
            }
        }
    }
}

impl std::error::Error for ConditionError {}

/// Convenience helpers for combining conditions.
///
/// Mirrors the original `anyMatches` and single-condition `matches` helpers.
pub fn any_matches(
    context: &dyn ApplicationContext,
    conditions: &[&dyn NextWebConditionExt],
) -> Result<bool, ConditionError> {
    for condition in conditions {
        if matches_one(context, *condition)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Evaluates a single condition, bypassing its public entry point when it is
/// a [`NextWebCondition`] so that logging and reporting are not duplicated.
pub fn matches_one(
    context: &dyn ApplicationContext,
    condition: &dyn NextWebConditionExt,
) -> Result<bool, ConditionError> {
    Ok(condition.get_match_outcome(context)?.is_match())
}
