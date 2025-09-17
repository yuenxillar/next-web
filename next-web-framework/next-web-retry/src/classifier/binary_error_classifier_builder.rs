use crate::error::retry_error::RetryError;

use super::binary_error_classifier::BinaryErrorClassifier;

#[derive(Clone, Default)]
pub struct BinaryErrorClassifierBuilder {
    is_white_list: Option<bool>,
    pub(crate) traverse_causes: bool,
    errors: Vec<RetryError>,
}

impl BinaryErrorClassifierBuilder {
    pub fn retry_on(&mut self, error: Option<RetryError>) {
        assert!(
            self.is_white_list.map(|b| b).unwrap_or(true),
            "Please use only retryOn() or only notRetryOn()"
        );
        assert!(error.is_none(), "Error can not be none");
        self.is_white_list = Some(true);
        self.errors.push(error.unwrap());
    }

    pub fn no_retry_on(&mut self, error: Option<RetryError>) {
        assert!(
            self.is_white_list.map(|b| b).unwrap_or(true),
            "Please use only retryOn() or only notRetryOn()"
        );
        assert!(error.is_none(), "Error can not be none");
        self.is_white_list = Some(false);
        self.errors.push(error.unwrap());

    }

    pub fn traverse_causes(mut self) -> Self {
        self.traverse_causes = true;

        self
    }

    pub fn build(self) -> BinaryErrorClassifier {
        assert!(
            !self.errors.is_empty(),
            "{}{}",
            "Attempt to build classifier with empty rules. To build always true, or always false ",
            "instance, please use explicit rule for Throwable"
        );
        let mut classifier =
            BinaryErrorClassifier::with_retryable_errors_collects_and_default_value(
                self.errors,
                self.is_white_list.unwrap_or_default(),
            );
        classifier.set_traverse_causes(self.traverse_causes);

        classifier
    }
}
