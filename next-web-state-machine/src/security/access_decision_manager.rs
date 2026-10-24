//! Access decisions for the security rules of events and transitions.

use std::collections::HashSet;

use next_web_core::any::AnyValue;
use next_web_core::error::BoxError;

use crate::security::security_rule::ComparisonType;
use crate::state_machine_exception::{StateMachineErrorKind, state_machine_error};

/// A configuration attribute of a security rule.
pub trait ConfigAttribute: Send + Sync {
    /// Gets the attribute value.
    fn get_attribute(&self) -> Option<&str>;
}

/// A plain configuration attribute.
pub struct SimpleConfigAttribute {
    attribute: String,
}

impl SimpleConfigAttribute {
    /// Creates a new attribute.
    ///
    /// # Arguments
    /// * `attribute` - the attribute value
    pub fn new(attribute: impl Into<String>) -> Self {
        Self {
            attribute: attribute.into(),
        }
    }
}

impl ConfigAttribute for SimpleConfigAttribute {
    fn get_attribute(&self) -> Option<&str> {
        Some(&self.attribute)
    }
}

/// Decides whether an authenticated caller may run an event or a transition.
pub trait AccessDecisionManager: Send + Sync {
    /// Decides the access.
    ///
    /// # Arguments
    /// * `authentication` - the authentication of the caller
    /// * `value` - the value the access is requested for
    /// * `config_attributes` - the attributes of the security rule
    fn decide(
        &self,
        authentication: &AnyValue,
        value: &AnyValue,
        config_attributes: Vec<Box<dyn ConfigAttribute>>,
    ) -> Result<(), BoxError>;

    /// Checks whether this manager understands the given attribute.
    ///
    /// # Arguments
    /// * `attribute` - the attribute to check
    fn supports(&self, attribute: &dyn ConfigAttribute) -> bool;
}

/// Decides access by comparing the rule attributes with the authorities of the
/// authentication.
///
/// This is the Rust equivalent of the attribute voters plus the affirmative
/// decision manager of Spring Security: the comparison type decides whether one,
/// all or most of the required attributes have to be granted.
pub struct DefaultAccessDecisionManager {
    comparison_type: ComparisonType,
}

impl DefaultAccessDecisionManager {
    /// Creates a manager with the given comparison type.
    ///
    /// # Arguments
    /// * `comparison_type` - whether one, all or most attributes are required
    pub fn new(comparison_type: ComparisonType) -> Self {
        Self { comparison_type }
    }

    /// Creates a manager which accepts any matching attribute.
    pub fn any() -> Self {
        Self::new(ComparisonType::Any)
    }
}

impl Default for DefaultAccessDecisionManager {
    fn default() -> Self {
        Self::any()
    }
}

impl AccessDecisionManager for DefaultAccessDecisionManager {
    fn decide(
        &self,
        authentication: &AnyValue,
        _value: &AnyValue,
        config_attributes: Vec<Box<dyn ConfigAttribute>>,
    ) -> Result<(), BoxError> {
        let required: HashSet<String> = config_attributes
            .iter()
            .filter_map(|attribute| attribute.get_attribute().map(str::to_owned))
            .collect();

        let authorities = authorities_of_authentication(authentication);

        if self.comparison_type.evaluate_access(&required, &authorities) {
            Ok(())
        } else {
            Err(state_machine_error(
                StateMachineErrorKind::AccessDenied,
                "the access decision manager denied the operation",
            ))
        }
    }

    fn supports(&self, attribute: &dyn ConfigAttribute) -> bool {
        attribute.get_attribute().is_some()
    }
}

/// Extracts the authorities granted by an authentication value.
///
/// The authentication may be a single authority, a list of authorities or a
/// `HashSet<String>`.
///
/// # Arguments
/// * `authentication` - the authentication value
pub fn authorities_of_authentication(authentication: &AnyValue) -> HashSet<String> {
    if let Some(authorities) = authentication.as_ref_value::<HashSet<String>>() {
        return authorities.clone();
    }

    if let Some(items) = authentication.as_vec() {
        return items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect();
    }

    if let Some(authority) = authentication.as_str() {
        return HashSet::from([authority.to_owned()]);
    }

    HashSet::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decide(
        comparison_type: ComparisonType,
        required: &[&str],
        authorities: &[&str],
    ) -> Result<(), BoxError> {
        let manager = DefaultAccessDecisionManager::new(comparison_type);
        let attributes: Vec<Box<dyn ConfigAttribute>> = required
            .iter()
            .map(|attribute| {
                Box::new(SimpleConfigAttribute::new(*attribute)) as Box<dyn ConfigAttribute>
            })
            .collect();

        let authentication = AnyValue::from(
            authorities
                .iter()
                .map(|authority| AnyValue::from(*authority))
                .collect::<Vec<_>>(),
        );

        manager.decide(&authentication, &authentication, attributes)
    }

    #[test]
    fn any_comparison_accepts_one_matching_attribute() {
        assert!(decide(ComparisonType::Any, &["admin", "user"], &["user"]).is_ok());
        assert!(decide(ComparisonType::Any, &["admin"], &["user"]).is_err());
        assert!(decide(ComparisonType::Any, &[], &[]).is_ok());
    }

    #[test]
    fn all_comparison_requires_every_attribute() {
        assert!(decide(ComparisonType::All, &["admin", "user"], &["user", "admin"]).is_ok());
        assert!(decide(ComparisonType::All, &["admin", "user"], &["user"]).is_err());
    }

    #[test]
    fn majority_comparison_requires_more_than_half() {
        assert!(decide(ComparisonType::Majority, &["a", "b", "c"], &["a", "b"]).is_ok());
        assert!(decide(ComparisonType::Majority, &["a", "b", "c"], &["a"]).is_err());
    }
}

