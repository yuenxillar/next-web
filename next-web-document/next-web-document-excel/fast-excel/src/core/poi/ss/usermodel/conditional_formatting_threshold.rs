use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};

/// The Threshold / CFVO / Conditional Formatting Value Object.
///
/// This defines how to calculate the ranges for a conditional
/// formatting rule, e.g., which values get a Green Traffic Light
/// icon and which Yellow or Red.
pub trait ConditionalFormattingThreshold: Debug {
    /// Get the Range Type used.
    ///
    /// # Returns
    /// The range type.
    fn get_range_type(&self) -> RangeType;

    /// Changes the Range Type used.
    ///
    /// If you change the range type, you need to
    /// ensure that the Formula and Value parameters
    /// are compatible with it before saving.
    ///
    /// # Arguments
    /// * `range_type` - The new range type.
    fn set_range_type(&mut self, range_type: RangeType);

    /// Formula to use to calculate the threshold.
    ///
    /// # Returns
    /// The formula, or `None` if no formula.
    fn get_formula(&self) -> Option<&str>;

    /// Sets the formula used to calculate the threshold,
    /// or unsets it if `None` is given.
    ///
    /// # Arguments
    /// * `formula` - The formula, or `None` to unset.
    fn set_formula(&mut self, formula: Option<String>);

    /// Gets the value used for the threshold.
    ///
    /// # Returns
    /// The value, or `None` if there isn't one.
    fn get_value(&self) -> Option<f64>;

    /// Sets the value used for the threshold.
    ///
    /// If the type is `RangeType::Percent` or
    /// `RangeType::Percentile` it must be between 0 and 100.
    /// If the type is `RangeType::Min` or `RangeType::Max`
    /// or `RangeType::Formula` it shouldn't be set.
    /// Use `None` to unset.
    ///
    /// # Arguments
    /// * `value` - The value, or `None` to unset.
    ///
    /// # Panics
    /// Panics if the value is invalid for the current range type.
    fn set_value(&mut self, value: Option<f64>);
}

/// Range type enumeration for conditional formatting thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeType {
    /// Number / Parameter
    Number = 1,

    /// The minimum value from the range
    Min = 2,

    /// The maximum value from the range
    Max = 3,

    /// Percent of the way from the min to the max value in the range
    Percent = 4,

    /// The minimum value of the cell that is in X percentile of the range
    Percentile = 5,

    /// Unallocated / reserved
    Unallocated = 6,

    /// Formula result
    Formula = 7,
}

impl RangeType {
    /// Get the numeric ID of the type.
    ///
    /// # Returns
    /// Numeric ID (1-based).
    pub fn get_id(&self) -> u8 {
        *self as u8
    }

    /// Get the system name of the type.
    ///
    /// # Returns
    /// System name, or `None` for unallocated.
    pub fn get_name(&self) -> Option<&'static str> {
        match self {
            RangeType::Number => Some("num"),
            RangeType::Min => Some("min"),
            RangeType::Max => Some("max"),
            RangeType::Percent => Some("percent"),
            RangeType::Percentile => Some("percentile"),
            RangeType::Unallocated => None,
            RangeType::Formula => Some("formula"),
        }
    }

    /// Get range type by numeric ID.
    ///
    /// # Arguments
    /// * `id` - Numeric ID (1-based).
    ///
    /// # Returns
    /// Range type, or `None` if ID is invalid.
    pub fn by_id(id: u8) -> Option<Self> {
        // id is mapped to ordinal()+1 (1-based IDs)
        if id == 0 || id > 7 {
            return None;
        }
        match id {
            1 => Some(RangeType::Number),
            2 => Some(RangeType::Min),
            3 => Some(RangeType::Max),
            4 => Some(RangeType::Percent),
            5 => Some(RangeType::Percentile),
            6 => Some(RangeType::Unallocated),
            7 => Some(RangeType::Formula),
            _ => None,
        }
    }

    /// Get range type by system name.
    ///
    /// # Arguments
    /// * `name` - System name.
    ///
    /// # Returns
    /// Range type, or `None` if name is invalid.
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "num" => Some(RangeType::Number),
            "min" => Some(RangeType::Min),
            "max" => Some(RangeType::Max),
            "percent" => Some(RangeType::Percent),
            "percentile" => Some(RangeType::Percentile),
            "formula" => Some(RangeType::Formula),
            _ => None,
        }
    }

    /// Check if this range type requires a numeric value.
    ///
    /// # Returns
    /// `true` if the type requires a numeric value.
    pub fn requires_value(&self) -> bool {
        matches!(
            self,
            RangeType::Number | RangeType::Percent | RangeType::Percentile
        )
    }

    /// Check if this range type requires a formula.
    ///
    /// # Returns
    /// `true` if the type requires a formula.
    pub fn requires_formula(&self) -> bool {
        matches!(self, RangeType::Formula)
    }

    /// Check if this range type uses special values (min/max).
    ///
    /// # Returns
    /// `true` if the type uses special min/max values.
    pub fn uses_special_value(&self) -> bool {
        matches!(self, RangeType::Min | RangeType::Max)
    }

    /// Validate a value for this range type.
    ///
    /// # Arguments
    /// * `value` - The value to validate.
    ///
    /// # Returns
    /// `Ok(())` if valid, or an error message if invalid.
    pub fn validate_value(&self, value: Option<f64>) -> Result<(), String> {
        match self {
            RangeType::Percent | RangeType::Percentile => match value {
                Some(v) if v >= 0.0 && v <= 100.0 => Ok(()),
                Some(_) => Err(format!("Value must be between 0 and 100 for {:?}", self)),
                None => Err(format!("Value is required for {:?}", self)),
            },
            RangeType::Number => match value {
                Some(_) => Ok(()),
                None => Err("Value is required for Number type".to_string()),
            },
            RangeType::Min | RangeType::Max | RangeType::Formula => {
                if value.is_some() {
                    Err(format!("Value should not be set for {:?} type", self))
                } else {
                    Ok(())
                }
            }
            RangeType::Unallocated => {
                Err("Unallocated range type is not valid for use".to_string())
            }
        }
    }

    /// Validate a formula for this range type.
    ///
    /// # Arguments
    /// * `formula` - The formula to validate.
    ///
    /// # Returns
    /// `Ok(())` if valid, or an error message if invalid.
    pub fn validate_formula(&self, formula: Option<&str>) -> Result<(), String> {
        match self {
            RangeType::Formula => match formula {
                Some(f) if !f.trim().is_empty() => Ok(()),
                _ => Err("Formula is required for Formula type".to_string()),
            },
            _ => {
                // Other types may have optional formulas
                Ok(())
            }
        }
    }
}

impl Display for RangeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let display_text = match self {
            RangeType::Number => "Number",
            RangeType::Min => "Minimum",
            RangeType::Max => "Maximum",
            RangeType::Percent => "Percent",
            RangeType::Percentile => "Percentile",
            RangeType::Unallocated => "Unallocated",
            RangeType::Formula => "Formula",
        };
        write!(f, "{}", display_text)
    }
}

impl TryFrom<u8> for RangeType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        RangeType::by_id(value).ok_or("Invalid range type ID")
    }
}

impl From<RangeType> for u8 {
    fn from(range_type: RangeType) -> Self {
        range_type.get_id()
    }
}
