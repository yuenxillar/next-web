//! Classifies an error by its Rust type and by its fields.
//!
//! The value-based classifier matches errors by kind and message, which is enough when a
//! configuration names a representative failure. A real service usually knows the failure's
//! *type* and often one of its fields: a `PaymentError` is retryable, an `ApiError` with status
//! 400 is not. This classifier expresses exactly that, without rendering the error to text.

use std::{any::Any, error::Error, marker::PhantomData, sync::Arc};

use next_web_core::any::AnyError;

use crate::{Predicate, error::retry_error::RetryError};

use super::classifier::Classifier;

/// The maximum depth walked when causes are traversed.
const MAX_CAUSE_DEPTH: usize = 32;

/// One inspectable view of an error.
enum Candidate<'a> {
    /// A type-erased error value, such as the retry error itself or its wrapped payload.
    Any(&'a dyn Any),
    /// A borrowed link of a `source()` chain.
    Link(&'a (dyn Error + 'static)),
}

impl Candidate<'_> {
    fn downcast<T>(&self) -> Option<&T>
    where
        T: AnyError + 'static,
    {
        match self {
            Candidate::Any(any) => any.downcast_ref::<T>(),
            Candidate::Link(link) => link.downcast_ref::<T>(),
        }
    }
}

/// One classification rule.
///
/// A rule answers only when it recognises the candidate; `None` means "no opinion", so later
/// rules and the default still apply.
trait ErrorRule
where
    Self: Send + Sync,
{
    fn evaluate(&self, candidate: &Candidate<'_>) -> Option<bool>;
}

/// Matches every error of one concrete type.
struct TypeRule<T> {
    answer: bool,
    marker: PhantomData<fn() -> T>,
}

impl<T> ErrorRule for TypeRule<T>
where
    T: AnyError + 'static,
{
    fn evaluate(&self, candidate: &Candidate<'_>) -> Option<bool> {
        candidate.downcast::<T>().map(|_| self.answer)
    }
}

/// Matches errors of one concrete type whose fields satisfy a predicate.
struct PredicateRule<T, F> {
    answer: bool,
    predicate: F,
    marker: PhantomData<fn() -> T>,
}

impl<T, F> ErrorRule for PredicateRule<T, F>
where
    T: AnyError + 'static,
    F: Fn(&T) -> bool + Send + Sync,
{
    fn evaluate(&self, candidate: &Candidate<'_>) -> Option<bool> {
        candidate
            .downcast::<T>()
            .filter(|value| (self.predicate)(value))
            .map(|_| self.answer)
    }
}

/// Classifies an error by its concrete type and, optionally, by its fields.
///
/// Rules are evaluated in the order they were added, and the first rule that recognises the
/// error decides. By default only the error itself is inspected: the [`RetryError`] and, when it
/// wraps a payload, that payload. Enabling cause traversal additionally inspects every link of
/// the error's `source()` chain.
///
/// # Example
///
/// ```
/// use next_web_retry::{
///     classifier::{Classifier, ErrorTypeClassifier},
///     error::retry_error::RetryError,
/// };
///
/// #[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// struct HttpError {
///     status: u16,
/// }
///
/// impl std::fmt::Display for HttpError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "http {}", self.status)
///     }
/// }
///
/// impl std::error::Error for HttpError {}
///
/// let mut classifier = ErrorTypeClassifier::new(true);
/// classifier.no_retry_on_if::<HttpError, _>(|error| error.status == 400);
///
/// let bad_request = RetryError::Any(Box::new(HttpError { status: 400 }));
/// let timeout = RetryError::Any(Box::new(HttpError { status: 504 }));
///
/// assert!(!classifier.classify(Some(&bad_request)));
/// assert!(classifier.classify(Some(&timeout)));
/// ```
pub struct ErrorTypeClassifier {
    rules: Vec<Box<dyn ErrorRule>>,
    default_value: bool,
    traverse_causes: bool,
}

impl ErrorTypeClassifier {
    /// Creates a classifier with the answer to give for errors no rule recognises.
    pub fn new(default_value: bool) -> Self {
        Self {
            rules: Vec::new(),
            default_value,
            traverse_causes: false,
        }
    }

    /// Creates a classifier that retries everything no rule rejects.
    pub fn retrying_by_default() -> Self {
        Self::new(true)
    }

    /// Creates a classifier that retries nothing no rule accepts.
    pub fn rejecting_by_default() -> Self {
        Self::new(false)
    }

    /// Enables cause traversal.
    ///
    /// Useful when a service wraps its own failures: only the root cause carries the type the
    /// rules talk about.
    pub fn with_traverse_causes(mut self) -> Self {
        self.traverse_causes = true;
        self
    }

    /// Enables or disables cause traversal.
    pub fn set_traverse_causes(&mut self, traverse_causes: bool) {
        self.traverse_causes = traverse_causes;
    }

    /// Returns whether cause traversal is enabled.
    pub fn is_traverse_causes(&self) -> bool {
        self.traverse_causes
    }

    /// Retries every error of type `T`.
    pub fn retry_on<T>(&mut self) -> &mut Self
    where
        T: AnyError + 'static,
    {
        self.rules.push(Box::new(TypeRule::<T> {
            answer: true,
            marker: PhantomData,
        }));
        self
    }

    /// Never retries an error of type `T`.
    pub fn no_retry_on<T>(&mut self) -> &mut Self
    where
        T: AnyError + 'static,
    {
        self.rules.push(Box::new(TypeRule::<T> {
            answer: false,
            marker: PhantomData,
        }));
        self
    }

    /// Retries errors of type `T` whose fields satisfy `predicate`.
    pub fn retry_on_if<T, F>(&mut self, predicate: F) -> &mut Self
    where
        T: AnyError + 'static,
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.rules.push(Box::new(PredicateRule::<T, F> {
            answer: true,
            predicate,
            marker: PhantomData,
        }));
        self
    }

    /// Never retries errors of type `T` whose fields satisfy `predicate`.
    pub fn no_retry_on_if<T, F>(&mut self, predicate: F) -> &mut Self
    where
        T: AnyError + 'static,
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.rules.push(Box::new(PredicateRule::<T, F> {
            answer: false,
            predicate,
            marker: PhantomData,
        }));
        self
    }

    /// Returns the answer given to errors no rule recognises.
    pub fn default_value(&self) -> bool {
        self.default_value
    }

    /// Returns the number of rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Returns whether no rule was added.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Returns every view of the error that rules may inspect.
    fn candidates<'a>(&self, error: &'a RetryError) -> Vec<Candidate<'a>> {
        let mut candidates = Vec::new();
        push_retry_error(error, &mut candidates);

        if !self.traverse_causes {
            return candidates;
        }

        let mut current = error.source();
        let mut depth = 0;
        while let Some(link) = current {
            if depth >= MAX_CAUSE_DEPTH {
                break;
            }
            depth += 1;

            if let Some(retry_error) = link.downcast_ref::<RetryError>() {
                push_retry_error(retry_error, &mut candidates);
                current = retry_error.source();
            } else {
                candidates.push(Candidate::Link(link));
                current = link.source();
            }
        }

        candidates
    }

    /// Converts the classifier into a predicate a retry policy can use.
    pub fn into_predicate(self) -> Arc<dyn Predicate<RetryError>> {
        Arc::new(TypeClassifierPredicate(self))
    }
}

impl Classifier<RetryError, bool> for ErrorTypeClassifier {
    fn classify(&self, classifiable: Option<&RetryError>) -> bool {
        let Some(error) = classifiable else {
            return self.default_value;
        };

        for candidate in self.candidates(error) {
            for rule in &self.rules {
                if let Some(answer) = rule.evaluate(&candidate) {
                    return answer;
                }
            }
        }

        self.default_value
    }
}

/// Adds the retry error itself and, when it wraps a payload, that payload.
fn push_retry_error<'a>(error: &'a RetryError, candidates: &mut Vec<Candidate<'a>>) {
    candidates.push(Candidate::Any(error));

    if let RetryError::Any(inner) = error {
        candidates.push(Candidate::Any(inner.as_any()));
    }
}

/// Adapts [`ErrorTypeClassifier`] to the predicate a retry policy consumes.
struct TypeClassifierPredicate(ErrorTypeClassifier);

impl Predicate<RetryError> for TypeClassifierPredicate {
    fn test(&self, value: &RetryError) -> bool {
        self.0.classify(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fmt,
        hash::{Hash, Hasher},
    };

    use super::ErrorTypeClassifier;
    use crate::{classifier::classifier::Classifier, error::retry_error::RetryError};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct HttpError {
        status: u16,
    }

    impl fmt::Display for HttpError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "http {}", self.status)
        }
    }

    impl std::error::Error for HttpError {}

    impl Hash for HttpError {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.status.hash(state);
        }
    }

    fn http_error(status: u16) -> RetryError {
        RetryError::Any(Box::new(HttpError { status }))
    }

    #[test]
    fn a_type_rule_decides_by_the_concrete_type() {
        let mut classifier = ErrorTypeClassifier::rejecting_by_default();
        classifier.retry_on::<HttpError>();

        assert!(classifier.classify(Some(&http_error(400))));
        assert!(!classifier.classify(Some(&RetryError::custom("text"))));
        assert!(!classifier.classify(None));
        assert_eq!(classifier.len(), 1);
    }

    #[test]
    fn a_field_rule_decides_by_the_payload() {
        let mut classifier = ErrorTypeClassifier::retrying_by_default();
        classifier.no_retry_on_if::<HttpError, _>(|error| error.status == 400);

        assert!(!classifier.classify(Some(&http_error(400))));
        assert!(classifier.classify(Some(&http_error(504))));
    }

    #[test]
    fn rules_fire_in_the_order_they_were_added() {
        let mut classifier = ErrorTypeClassifier::retrying_by_default();
        classifier.retry_on_if::<HttpError, _>(|error| error.status == 503);
        classifier.no_retry_on::<HttpError>();

        assert!(
            classifier.classify(Some(&http_error(503))),
            "the first matching rule must win"
        );
        assert!(!classifier.classify(Some(&http_error(500))));
    }

    #[test]
    fn cause_traversal_finds_the_wrapped_type() {
        let mut shallow = ErrorTypeClassifier::rejecting_by_default();
        shallow.retry_on::<HttpError>();
        let mut deep = ErrorTypeClassifier::rejecting_by_default();
        deep.retry_on::<HttpError>();
        deep.set_traverse_causes(true);

        let wrapped = RetryError::retry_with_cause("outer", Box::new(http_error(503)));

        assert!(!shallow.classify(Some(&wrapped)));
        assert!(deep.classify(Some(&wrapped)));
    }

    #[test]
    fn a_deep_retry_error_chain_is_traversed() {
        let mut classifier = ErrorTypeClassifier::rejecting_by_default();
        classifier.retry_on::<HttpError>();
        classifier.set_traverse_causes(true);

        let nested = RetryError::retry_with_cause(
            "outer",
            Box::new(RetryError::retry_with_cause(
                "middle",
                Box::new(http_error(429)),
            )),
        );

        assert!(classifier.classify(Some(&nested)));
    }

    #[test]
    fn the_classifier_converts_into_a_predicate() {
        let mut classifier = ErrorTypeClassifier::rejecting_by_default();
        classifier.retry_on::<HttpError>();
        let predicate = classifier.into_predicate();

        assert!(predicate.test(&http_error(500)));
        assert!(!predicate.test(&RetryError::custom("text")));
    }

    #[test]
    fn an_error_without_rules_uses_the_default() {
        let retrying = ErrorTypeClassifier::retrying_by_default();
        let rejecting = ErrorTypeClassifier::rejecting_by_default();

        assert!(retrying.classify(Some(&http_error(400))));
        assert!(!rejecting.classify(Some(&http_error(400))));
        assert!(retrying.is_empty());
        assert!(retrying.default_value());
    }
}
