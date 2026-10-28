//! Error classification.
//!
//! Classifiers answer questions about errors. The retry policies are built on them:
//! [`binary_error_classifier::BinaryErrorClassifier`] decides whether an error is retryable by
//! value, [`error_type_classifier::ErrorTypeClassifier`] decides by Rust type and fields, and
//! [`map_classifier::MapClassifier`] decides which policy applies to an error.

pub mod binary_error_classifier;
pub mod binary_error_classifier_builder;
// The classifier module is named after the concept it defines, which matches the original
// package layout; the repetition is intentional.
#[allow(clippy::module_inception)]
pub mod classifier;
pub mod error_type_classifier;
pub mod map_classifier;

pub use binary_error_classifier::BinaryErrorClassifier;
pub use binary_error_classifier_builder::BinaryErrorClassifierBuilder;
pub use classifier::{Classifier, FnClassifier};
pub use error_type_classifier::ErrorTypeClassifier;
pub use map_classifier::{BoxedClassifier, ConstantClassifier, MapClassifier};
