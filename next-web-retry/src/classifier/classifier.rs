//! The classifier abstraction the retry policies are built on.

use crate::error::retry_error::RetryError;

/// Maps a value to another value through a lookup or a rule.
///
/// Classifiers are the extension point the policies use to decide what to do with an
/// error: one policy asks a binary classifier "retry or not", another asks which policy to
/// apply.
pub trait Classifier<T, C>
where
    Self: Send + Sync,
{
    /// Classifies `classifiable`, falling back to the classifier's default when it is
    /// `None` or unknown.
    fn classify(&self, classifiable: Option<&T>) -> C;
}

/// Adapts a closure to [`Classifier`].
pub struct FnClassifier<F> {
    callback: F,
    default: bool,
}

impl<F> FnClassifier<F>
where
    F: Fn(&RetryError) -> bool + Send + Sync,
{
    /// Creates a classifier from a closure, using `default` for missing errors.
    pub fn new(default: bool, callback: F) -> Self {
        Self { callback, default }
    }
}

impl<F> Classifier<RetryError, bool> for FnClassifier<F>
where
    F: Fn(&RetryError) -> bool + Send + Sync,
{
    fn classify(&self, classifiable: Option<&RetryError>) -> bool {
        classifiable.map_or(self.default, |error| (self.callback)(error))
    }
}

#[cfg(test)]
mod tests {
    use super::{Classifier, FnClassifier};
    use crate::error::retry_error::RetryError;

    #[test]
    fn a_closure_classifier_uses_the_default_for_a_missing_error() {
        let classifier = FnClassifier::new(false, |_: &RetryError| true);

        assert!(!classifier.classify(None));
        assert!(classifier.classify(Some(&RetryError::custom("any"))));
    }
}
