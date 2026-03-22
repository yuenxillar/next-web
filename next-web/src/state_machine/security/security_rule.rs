use std::collections::HashSet;

/// Encapsulates the rules for comparing security attributes and expression.
///
/// This struct defines security rules for state machine transitions and events,
/// including attributes, comparison types, and security expressions.
#[derive(Debug, Clone)]
pub struct SecurityRule {
    /// Security attributes required for authorization
    attributes: Option<HashSet<String>>,
    /// Comparison type for evaluating multiple attributes
    comparison_type: ComparisonType,
    /// Security expression for custom authorization logic
    expression: Option<String>,
}

impl SecurityRule {
    /// Creates a new security rule with default values.
    ///
    /// # Returns
    /// A new `SecurityRule` instance
    pub fn new() -> Self {
        Self {
            attributes: None,
            comparison_type: ComparisonType::Any,
            expression: None,
        }
    }

    /// Convert attributes to comma separated String.
    ///
    /// # Arguments
    /// * `attributes` - the attributes to convert
    ///
    /// # Returns
    /// Comma separated String
    pub fn security_attributes_to_comma_delimited_list(attributes: &[String]) -> String {
        attributes.join(", ")
    }

    /// Convert attributes from comma separated String to Collection.
    ///
    /// # Arguments
    /// * `attributes` - the attributes to convert
    ///
    /// # Returns
    /// Collection of parsed attributes
    pub fn comma_delimited_list_to_security_attributes(attributes: &str) -> HashSet<String> {
        attributes
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }

    /// Gets the security attributes.
    ///
    /// # Returns
    /// A reference to the security attributes, if any
    pub fn attributes(&self) -> Option<&HashSet<String>> {
        self.attributes.as_ref()
    }

    /// Sets the security attributes.
    ///
    /// # Arguments
    /// * `attributes` - the new security attributes
    pub fn set_attributes(&mut self, attributes: HashSet<String>) {
        self.attributes = Some(attributes);
    }

    /// Gets the comparison type.
    ///
    /// # Returns
    /// The comparison type
    pub fn comparison_type(&self) -> ComparisonType {
        self.comparison_type
    }

    /// Sets the comparison type.
    ///
    /// # Arguments
    /// * `comparison_type` - the new comparison type
    pub fn set_comparison_type(&mut self, comparison_type: ComparisonType) {
        self.comparison_type = comparison_type;
    }

    /// Gets the security expression.
    ///
    /// # Returns
    /// A reference to the security expression, if any
    pub fn expression(&self) -> Option<&str> {
        self.expression.as_deref()
    }

    /// Sets the security expression.
    ///
    /// # Arguments
    /// * `expression` - the new security expression
    pub fn set_expression(&mut self, expression: String) {
        self.expression = Some(expression);
    }

    /// Checks if the rule has attributes.
    ///
    /// # Returns
    /// `true` if the rule has attributes, `false` otherwise
    pub fn has_attributes(&self) -> bool {
        self.attributes.as_ref().map_or(false, |a| !a.is_empty())
    }

    /// Checks if the rule has an expression.
    ///
    /// # Returns
    /// `true` if the rule has an expression, `false` otherwise
    pub fn has_expression(&self) -> bool {
        self.expression.is_some()
    }

    /// Builder method to set attributes.
    ///
    /// # Arguments
    /// * `attributes` - the security attributes
    ///
    /// # Returns
    /// The security rule for method chaining
    pub fn with_attributes(mut self, attributes: HashSet<String>) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Builder method to set comparison type.
    ///
    /// # Arguments
    /// * `comparison_type` - the comparison type
    ///
    /// # Returns
    /// The security rule for method chaining
    pub fn with_comparison_type(mut self, comparison_type: ComparisonType) -> Self {
        self.comparison_type = comparison_type;
        self
    }

    /// Builder method to set expression.
    ///
    /// # Arguments
    /// * `expression` - the security expression
    ///
    /// # Returns
    /// The security rule for method chaining
    pub fn with_expression(mut self, expression: impl Into<String>) -> Self {
        self.expression = Some(expression.into());
        self
    }
}

impl Default for SecurityRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Security comparison types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonType {
    /// Compare method where any attribute authorization allows access
    Any,
    /// Compare method where all attribute authorization allows access
    All,
    /// Compare method where majority attribute authorization allows access
    Majority,
}

impl ComparisonType {
    /// Evaluates whether access should be granted based on the comparison type
    /// and the provided attributes.
    ///
    /// # Arguments
    /// * `required_attributes` - The attributes required by the security rule
    /// * `user_attributes` - The attributes possessed by the user
    ///
    /// # Returns
    /// `true` if access should be granted, `false` otherwise
    pub fn evaluate_access(
        &self,
        required_attributes: &HashSet<String>,
        user_attributes: &HashSet<String>,
    ) -> bool {
        if required_attributes.is_empty() {
            return true;
        }

        match self {
            ComparisonType::Any => {
                // Any match grants access
                required_attributes
                    .iter()
                    .any(|attr| user_attributes.contains(attr))
            }
            ComparisonType::All => {
                // All matches required
                required_attributes.is_subset(user_attributes)
            }
            ComparisonType::Majority => {
                // Majority of attributes must match
                if required_attributes.is_empty() {
                    return true;
                }
                let match_count = required_attributes
                    .iter()
                    .filter(|attr| user_attributes.contains(*attr))
                    .count();
                match_count > required_attributes.len() / 2
            }
        }
    }

    /// Gets the string representation of the comparison type.
    ///
    /// # Returns
    /// String representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ComparisonType::Any => "ANY",
            ComparisonType::All => "ALL",
            ComparisonType::Majority => "MAJORITY",
        }
    }
}

impl std::fmt::Display for ComparisonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
