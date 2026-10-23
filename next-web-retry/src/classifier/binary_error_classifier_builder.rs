//! Builds a binary error classifier from a white or black list.

use crate::error::retry_error::RetryError;

use super::binary_error_classifier::BinaryErrorClassifier;

/// Builds a [`BinaryErrorClassifier`] from a white or black list of errors.
///
/// The two styles are mutually exclusive: either every listed error is retried and
/// everything else is not, or every listed error is not retried and everything else is.
/// Mixing them is a configuration mistake, so the builder rejects it.
#[derive(Clone, Default)]
pub struct BinaryErrorClassifierBuilder {
    is_white_list: Option<bool>,
    pub(crate) traverse_causes: bool,
    errors: Vec<RetryError>,
}

impl BinaryErrorClassifierBuilder {
    /// Adds an error that must be retried.
    ///
    /// # Panics
    ///
    /// Configuration is validated eagerly, so this panics when [`Self::no_retry_on`] was
    /// used before. Configuration errors are startup errors and must not be discovered at
    /// runtime.
    pub fn retry_on(&mut self, error: RetryError) {
        assert!(
            self.is_white_list.unwrap_or(true),
            "Please use only retry_on() or only no_retry_on()"
        );
        self.is_white_list = Some(true);
        self.errors.push(error);
    }

    /// Adds an error that must not be retried.
    ///
    /// # Panics
    ///
    /// Configuration is validated eagerly, so this panics when [`Self::retry_on`] was used
    /// before.
    pub fn no_retry_on(&mut self, error: RetryError) {
        assert!(
            !self.is_white_list.unwrap_or(false),
            "Please use only retry_on() or only no_retry_on()"
        );
        self.is_white_list = Some(false);
        self.errors.push(error);
    }

    /// Enables cause traversal in the built classifier.
    pub fn traverse_causes(mut self) -> Self {
        self.traverse_causes = true;
        self
    }

    /// Builds the classifier.
    ///
    /// # Panics
    ///
    /// Panics when no rule was added: an empty rule set has no meaningful default, so it is
    /// rejected at configuration time rather than silently returning one answer forever.
    pub fn build(self) -> BinaryErrorClassifier {
        assert!(
            !self.errors.is_empty(),
            "{}{}",
            "Attempt to build classifier with empty rules. To build always true, or always false ",
            "instance, please use explicit rule for the error type"
        );
        let is_white_list = self.is_white_list.unwrap_or_default();
        let mut classifier =
            BinaryErrorClassifier::with_retryable_errors_collects_and_default_value(
                self.errors,
                !is_white_list,
            );
        classifier.set_traverse_causes(self.traverse_causes);

        classifier
    }
}

#[cfg(test)]
mod tests {
    use super::BinaryErrorClassifierBuilder;
    use crate::{classifier::classifier::Classifier, error::retry_error::RetryError};

    #[test]
    fn a_white_list_retries_only_the_listed_errors() {
        let mut builder = BinaryErrorClassifierBuilder::default();
        builder.retry_on(RetryError::custom("retry"));
        let classifier = builder.build();

        assert!(classifier.classify(Some(&RetryError::custom("retry"))));
        assert!(!classifier.classify(Some(&RetryError::custom("stop"))));
    }

    #[test]
    fn a_black_list_rejects_only_the_listed_errors() {
        let mut builder = BinaryErrorClassifierBuilder::default();
        builder.no_retry_on(RetryError::custom("stop"));
        let classifier = builder.build();

        assert!(!classifier.classify(Some(&RetryError::custom("stop"))));
        assert!(classifier.classify(Some(&RetryError::custom("retry"))));
    }

    #[test]
    #[should_panic(expected = "Please use only retry_on() or only no_retry_on()")]
    fn mixing_white_and_black_lists_is_rejected() {
        let mut builder = BinaryErrorClassifierBuilder::default();
        builder.retry_on(RetryError::custom("retry"));
        builder.no_retry_on(RetryError::custom("stop"));
    }

    #[test]
    #[should_panic(expected = "Attempt to build classifier with empty rules")]
    fn building_without_rules_is_rejected() {
        let _ = BinaryErrorClassifierBuilder::default().build();
    }
}
