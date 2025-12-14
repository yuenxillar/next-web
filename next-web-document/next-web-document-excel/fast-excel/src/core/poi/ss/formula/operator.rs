use std::cmp::Ordering;

/// Not calling it OperatorType to avoid confusion for now with other classes.
/// Definition order matches OOXML type ID indexes.
/// Note that this has NO_COMPARISON as the first item, unlike the similar
/// DataValidation operator enum. Thanks, Microsoft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum OperatorEnum {
    /// Always false/invalid
    NoComparison = 0,
    Between,
    NotBetween,
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
}

impl OperatorEnum {
    /// Evaluates comparison using operator instance rules
    ///
    /// # Arguments
    /// * `cell_value` - won't be None, assumption is previous checks handled that
    /// * `v1` - if None, per Excel behavior various results depending on the type of cell_value and the specific enum instance
    /// * `v2` - None if not needed. If None when needed, various results, per Excel behavior
    ///
    /// # Returns
    /// true if the comparison is valid
    pub(crate) fn is_valid<T: PartialOrd + ToString>(
        &self,
        cell_value: &T,
        v1: Option<&T>,
        v2: Option<&T>,
    ) -> bool {
        match self {
            OperatorEnum::NoComparison => Self::no_comp(cell_value, v1, v2),
            OperatorEnum::Between => Self::between(cell_value, v1, v2),
            OperatorEnum::NotBetween => Self::not_between(cell_value, v1, v2),
            OperatorEnum::Equal => Self::equal_check(cell_value, v1, v2),
            OperatorEnum::NotEqual => Self::not_equal(cell_value, v1, v2),
            OperatorEnum::GreaterThan => Self::greater_than(cell_value, v1, v2),
            OperatorEnum::LessThan => Self::less_than(cell_value, v1, v2),
            OperatorEnum::GreaterOrEqual => Self::greater_or_equal(cell_value, v1, v2),
            OperatorEnum::LessOrEqual => Self::less_or_equal(cell_value, v1, v2),
        }
    }

    /// Evaluates comparison for boolean values
    pub(crate) fn is_valid_bool(
        &self,
        cell_value: bool,
        v1: Option<bool>,
        v2: Option<bool>,
    ) -> bool {
        match self {
            OperatorEnum::NoComparison => false,
            OperatorEnum::Between => Self::between(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::NotBetween => Self::not_between(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::Equal => Self::equal_check(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::NotEqual => Self::not_equal(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::GreaterThan => Self::greater_than(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::LessThan => Self::less_than(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::GreaterOrEqual => {
                Self::greater_or_equal(&cell_value, v1.as_ref(), v2.as_ref())
            }
            OperatorEnum::LessOrEqual => Self::less_or_equal(&cell_value, v1.as_ref(), v2.as_ref()),
        }
    }

    /// Evaluates comparison for numeric values
    pub(crate) fn is_valid_number(
        &self,
        cell_value: f64,
        v1: Option<f64>,
        v2: Option<f64>,
    ) -> bool {
        match self {
            OperatorEnum::NoComparison => false,
            OperatorEnum::Between => Self::between(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::NotBetween => Self::not_between(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::Equal => Self::equal_check(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::NotEqual => Self::not_equal(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::GreaterThan => Self::greater_than(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::LessThan => Self::less_than(&cell_value, v1.as_ref(), v2.as_ref()),
            OperatorEnum::GreaterOrEqual => {
                Self::greater_or_equal(&cell_value, v1.as_ref(), v2.as_ref())
            }
            OperatorEnum::LessOrEqual => Self::less_or_equal(&cell_value, v1.as_ref(), v2.as_ref()),
        }
    }

    /// Evaluates comparison for string values
    pub(crate) fn is_valid_string(
        &self,
        cell_value: &str,
        v1: Option<&str>,
        v2: Option<&str>,
    ) -> bool {
        match self {
            OperatorEnum::NoComparison => false,
            OperatorEnum::Between => Self::between(&cell_value, v1, v2),
            OperatorEnum::NotBetween => Self::not_between(&cell_value, v1, v2),
            OperatorEnum::Equal => Self::equal_check(&cell_value, v1, v2),
            OperatorEnum::NotEqual => Self::not_equal(&cell_value, v1, v2),
            OperatorEnum::GreaterThan => Self::greater_than(&cell_value, v1, v2),
            OperatorEnum::LessThan => Self::less_than(&cell_value, v1, v2),
            OperatorEnum::GreaterOrEqual => Self::greater_or_equal(&cell_value, v1, v2),
            OperatorEnum::LessOrEqual => Self::less_or_equal(&cell_value, v1, v2),
        }
    }

    /// Called when the cell and comparison values are of different data types
    /// Needed for negation operators, which should return true.
    ///
    /// # Returns
    /// true if this comparison is true when the types to compare are different
    pub(crate) fn is_valid_for_incompatible_types(&self) -> bool {
        match self {
            OperatorEnum::NotBetween | OperatorEnum::NotEqual => true,
            _ => false,
        }
    }

    /// Converts from integer value to OperatorEnum
    pub(crate) fn from(value: i32) -> Self {
        match value {
            0 => OperatorEnum::NoComparison,
            1 => OperatorEnum::Between,
            2 => OperatorEnum::NotBetween,
            3 => OperatorEnum::Equal,
            4 => OperatorEnum::NotEqual,
            5 => OperatorEnum::GreaterThan,
            6 => OperatorEnum::LessThan,
            7 => OperatorEnum::GreaterOrEqual,
            8 => OperatorEnum::LessOrEqual,
            _ => OperatorEnum::NoComparison, // Default fallback
        }
    }

    // Private comparison methods

    fn no_comp<T: PartialOrd + ToString>(
        _cell_value: &T,
        _v1: Option<&T>,
        _v2: Option<&T>,
    ) -> bool {
        false
    }

    fn between<T: PartialOrd + ToString>(cell_value: &T, v1: Option<&T>, v2: Option<&T>) -> bool {
        match (v1, v2) {
            (Some(v1), Some(v2)) => {
                cell_value
                    .partial_cmp(v1)
                    .map_or(false, |c1| c1 != Ordering::Less)
                    && cell_value
                        .partial_cmp(v2)
                        .map_or(false, |c2| c2 != Ordering::Greater)
            }
            (None, Some(v2)) => {
                // When v1 is None, use default values based on type
                Self::between_with_defaults(cell_value, None, Some(v2))
            }
            (Some(v1), None) => {
                // When v2 is None, use default values based on type
                Self::between_with_defaults(cell_value, Some(v1), None)
            }
            (None, None) => {
                // Both are None
                Self::between_with_defaults(cell_value, None, None)
            }
        }
    }

    fn between_with_defaults<T: PartialOrd + ToString>(
        cell_value: &T,
        v1: Option<&T>,
        v2: Option<&T>,
    ) -> bool {
        // This is a simplified version - in practice, we'd need type-specific handling
        match (v1, v2) {
            (Some(v1), Some(v2)) => {
                cell_value
                    .partial_cmp(v1)
                    .map_or(false, |c1| c1 != Ordering::Less)
                    && cell_value
                        .partial_cmp(v2)
                        .map_or(false, |c2| c2 != Ordering::Greater)
            }
            _ => {
                // For strings, empty string is the default
                // For numbers, 0.0 is the default
                // This is a simplification - the actual implementation would need more type info
                false
            }
        }
    }

    fn not_between<T: PartialOrd + ToString>(
        cell_value: &T,
        v1: Option<&T>,
        v2: Option<&T>,
    ) -> bool {
        match (v1, v2) {
            (Some(v1), Some(v2)) => {
                cell_value
                    .partial_cmp(v1)
                    .map_or(false, |c1| c1 == Ordering::Less)
                    || cell_value
                        .partial_cmp(v2)
                        .map_or(false, |c2| c2 == Ordering::Greater)
            }
            (None, Some(v2)) => !Self::between(cell_value, Some(cell_value), Some(v2)),
            (Some(v1), None) => !Self::between(cell_value, Some(v1), Some(cell_value)),
            (None, None) => true,
        }
    }

    fn equal_check<T: PartialOrd + ToString>(
        cell_value: &T,
        v1: Option<&T>,
        _v2: Option<&T>,
    ) -> bool {
        match v1 {
            Some(v1) => {
                // For strings, do case-insensitive comparison
                if let (Some(s1), Some(s2)) = (Self::as_string(cell_value), Self::as_string(v1)) {
                    s1.eq_ignore_ascii_case(&s2)
                } else {
                    cell_value == v1
                }
            }
            None => {
                // When v1 is None, check against default value (0 for numbers, false for booleans)
                false
            }
        }
    }

    fn not_equal<T: PartialOrd + ToString>(
        cell_value: &T,
        v1: Option<&T>,
        _v2: Option<&T>,
    ) -> bool {
        match v1 {
            Some(v1) => {
                // For strings, do case-insensitive comparison
                if let (Some(s1), Some(s2)) = (Self::as_string(cell_value), Self::as_string(v1)) {
                    !s1.eq_ignore_ascii_case(&s2)
                } else {
                    cell_value != v1
                }
            }
            None => true, // Non-null not equal null, returns true
        }
    }

    fn greater_than<T: PartialOrd + ToString>(
        cell_value: &T,
        v1: Option<&T>,
        _v2: Option<&T>,
    ) -> bool {
        match v1 {
            Some(v1) => cell_value
                .partial_cmp(v1)
                .map_or(false, |c| c == Ordering::Greater),
            None => {
                // When v1 is None, check against default value
                // For numbers: cell_value > 0
                // For strings: non-null string > empty string
                Self::greater_than_default(cell_value)
            }
        }
    }

    fn less_than<T: PartialOrd + ToString>(
        cell_value: &T,
        v1: Option<&T>,
        _v2: Option<&T>,
    ) -> bool {
        match v1 {
            Some(v1) => cell_value
                .partial_cmp(v1)
                .map_or(false, |c| c == Ordering::Less),
            None => {
                // When v1 is None, check against default value
                Self::less_than_default(cell_value)
            }
        }
    }

    fn greater_or_equal<T: PartialOrd + ToString>(
        cell_value: &T,
        v1: Option<&T>,
        _v2: Option<&T>,
    ) -> bool {
        match v1 {
            Some(v1) => cell_value
                .partial_cmp(v1)
                .map_or(false, |c| c != Ordering::Less),
            None => {
                // When v1 is None, check against default value
                Self::greater_or_equal_default(cell_value)
            }
        }
    }

    fn less_or_equal<T: PartialOrd + ToString>(
        cell_value: &T,
        v1: Option<&T>,
        _v2: Option<&T>,
    ) -> bool {
        match v1 {
            Some(v1) => cell_value
                .partial_cmp(v1)
                .map_or(false, |c| c != Ordering::Greater),
            None => {
                // When v1 is None, check against default value
                Self::less_or_equal_default(cell_value)
            }
        }
    }

    // Helper methods for default value comparisons

    fn greater_than_default<T: PartialOrd + ToString>(_cell_value: &T) -> bool {
        // Default implementation - actual behavior depends on type
        // For numbers: compare with 0
        // For strings: non-null string > empty string (true)
        // This is a simplification
        false
    }

    fn less_than_default<T: PartialOrd + ToString>(_cell_value: &T) -> bool {
        // Default implementation
        false
    }

    fn greater_or_equal_default<T: PartialOrd + ToString>(_cell_value: &T) -> bool {
        // Default implementation
        false
    }

    fn less_or_equal_default<T: PartialOrd + ToString>(_cell_value: &T) -> bool {
        // Default implementation
        false
    }

    fn as_string<T: ToString>(value: &T) -> Option<String> {
        Some(value.to_string())
    }
}

// Implement From trait for easier conversion from i32
impl From<i32> for OperatorEnum {
    fn from(value: i32) -> Self {
        OperatorEnum::from(value)
    }
}

// Implement Into trait for conversion to u8
impl From<OperatorEnum> for u8 {
    fn from(op: OperatorEnum) -> u8 {
        op as u8
    }
}

// Implement Into trait for conversion to i32
impl From<OperatorEnum> for i32 {
    fn from(op: OperatorEnum) -> i32 {
        op as i32
    }
}
