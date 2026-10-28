use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};
use std::sync::OnceLock;

/// Represents a type of a conditional formatting rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionType {
    /// This conditional formatting rule compares a cell value
    /// to a formula calculated result, using an operator.
    CellValueIs = 1,

    /// This conditional formatting rule contains a formula to evaluate.
    /// When the formula result is true, the cell is highlighted.
    Formula = 2,

    /// This conditional formatting rule contains a color scale,
    /// with the cell background set according to a gradient.
    ColorScale = 3,

    /// This conditional formatting rule sets a data bar, with the
    /// cell populated with bars based on their values.
    DataBar = 4,

    /// This conditional formatting rule that filters the values.
    Filter = 5,

    /// This conditional formatting rule sets an icon set, with the
    /// cell populated with icons based on their values.
    IconSet = 6,
}

impl ConditionType {
    /// Get the numeric ID of the condition type.
    ///
    /// # Returns
    /// Numeric ID (1-6).
    pub fn get_id(&self) -> u8 {
        *self as u8
    }

    /// Get the XML type name.
    ///
    /// # Returns
    /// XML type name, or `None` for Filter type.
    pub fn get_type_name(&self) -> Option<&'static str> {
        match self {
            ConditionType::CellValueIs => Some("cellIs"),
            ConditionType::Formula => Some("expression"),
            ConditionType::ColorScale => Some("colorScale"),
            ConditionType::DataBar => Some("dataBar"),
            ConditionType::Filter => None,
            ConditionType::IconSet => Some("iconSet"),
        }
    }

    /// Get condition type by numeric ID.
    ///
    /// # Arguments
    /// * `id` - Numeric ID (1-6).
    ///
    /// # Returns
    /// Condition type, or `None` if ID is invalid.
    pub fn for_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(ConditionType::CellValueIs),
            2 => Some(ConditionType::Formula),
            3 => Some(ConditionType::ColorScale),
            4 => Some(ConditionType::DataBar),
            5 => Some(ConditionType::Filter),
            6 => Some(ConditionType::IconSet),
            _ => None,
        }
    }

    /// Get condition type by XML type name.
    ///
    /// # Arguments
    /// * `type_name` - XML type name.
    ///
    /// # Returns
    /// Condition type, or `None` if name is invalid.
    pub fn for_type_name(type_name: &str) -> Option<Self> {
        match type_name {
            "cellIs" => Some(ConditionType::CellValueIs),
            "expression" => Some(ConditionType::Formula),
            "colorScale" => Some(ConditionType::ColorScale),
            "dataBar" => Some(ConditionType::DataBar),
            "iconSet" => Some(ConditionType::IconSet),
            _ => None,
        }
    }

    /// Get the display name of the condition type.
    ///
    /// # Returns
    /// Human-readable display name.
    pub fn get_display_name(&self) -> &'static str {
        match self {
            ConditionType::CellValueIs => "Cell Value Is",
            ConditionType::Formula => "Formula",
            ConditionType::ColorScale => "Color Scale",
            ConditionType::DataBar => "Data Bar",
            ConditionType::Filter => "Filter",
            ConditionType::IconSet => "Icon Set",
        }
    }

    /// Check if this condition type uses a formula.
    ///
    /// # Returns
    /// `true` if the condition type uses a formula.
    pub fn uses_formula(&self) -> bool {
        matches!(self, ConditionType::CellValueIs | ConditionType::Formula)
    }

    /// Check if this condition type uses visual formatting.
    ///
    /// # Returns
    /// `true` if the condition type uses visual formatting (color scale, data bar, icon set).
    pub fn uses_visual_formatting(&self) -> bool {
        matches!(
            self,
            ConditionType::ColorScale | ConditionType::DataBar | ConditionType::IconSet
        )
    }

    /// Check if this condition type is a filter.
    ///
    /// # Returns
    /// `true` if the condition type is a filter.
    pub fn is_filter(&self) -> bool {
        matches!(self, ConditionType::Filter)
    }

    /// Check if this condition type requires thresholds.
    ///
    /// # Returns
    /// `true` if the condition type requires thresholds.
    pub fn requires_thresholds(&self) -> bool {
        matches!(
            self,
            ConditionType::ColorScale | ConditionType::DataBar | ConditionType::IconSet
        )
    }

    /// Get all available condition types.
    ///
    /// # Returns
    /// Vector of all condition types.
    pub fn all_types() -> Vec<ConditionType> {
        vec![
            ConditionType::CellValueIs,
            ConditionType::Formula,
            ConditionType::ColorScale,
            ConditionType::DataBar,
            ConditionType::Filter,
            ConditionType::IconSet,
        ]
    }

    /// Get condition types that support visual formatting.
    ///
    /// # Returns
    /// Vector of visual condition types.
    pub fn visual_types() -> Vec<ConditionType> {
        vec![
            ConditionType::ColorScale,
            ConditionType::DataBar,
            ConditionType::IconSet,
        ]
    }

    /// Get condition types that use formulas.
    ///
    /// # Returns
    /// Vector of formula-based condition types.
    pub fn formula_types() -> Vec<ConditionType> {
        vec![ConditionType::CellValueIs, ConditionType::Formula]
    }
}

impl Display for ConditionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.get_id(), self.get_display_name())
    }
}

impl TryFrom<u8> for ConditionType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        ConditionType::for_id(value).ok_or("Invalid condition type ID")
    }
}

impl From<ConditionType> for u8 {
    fn from(condition_type: ConditionType) -> Self {
        condition_type.get_id()
    }
}

/// Registry for condition types (for backward compatibility with Java-style static initialization).
pub struct ConditionTypeRegistry {
    lookup: HashMap<u8, ConditionType>,
}

impl ConditionTypeRegistry {
    /// Get the global condition type registry.
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<ConditionTypeRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let mut lookup = HashMap::new();
            for condition_type in ConditionType::all_types() {
                lookup.insert(condition_type.get_id(), condition_type);
            }
            ConditionTypeRegistry { lookup }
        })
    }

    /// Get condition type by ID.
    pub fn for_id(&self, id: u8) -> Option<ConditionType> {
        self.lookup.get(&id).copied()
    }

    /// Get all condition types.
    pub fn all(&self) -> Vec<ConditionType> {
        self.lookup.values().copied().collect()
    }
}

// Constants for backward compatibility
impl ConditionType {
    /// This conditional formatting rule compares a cell value
    /// to a formula calculated result, using an operator.
    pub const CELL_VALUE_IS: ConditionType = ConditionType::CellValueIs;

    /// This conditional formatting rule contains a formula to evaluate.
    /// When the formula result is true, the cell is highlighted.
    pub const FORMULA: ConditionType = ConditionType::Formula;

    /// This conditional formatting rule contains a color scale,
    /// with the cell background set according to a gradient.
    pub const COLOR_SCALE: ConditionType = ConditionType::ColorScale;

    /// This conditional formatting rule sets a data bar, with the
    /// cell populated with bars based on their values.
    pub const DATA_BAR: ConditionType = ConditionType::DataBar;

    /// This conditional formatting rule that filters the values.
    pub const FILTER: ConditionType = ConditionType::Filter;

    /// This conditional formatting rule sets an icon set, with the
    /// cell populated with icons based on their values.
    pub const ICON_SET: ConditionType = ConditionType::IconSet;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_type_conversions() {
        // Test ID conversions
        assert_eq!(ConditionType::CellValueIs.get_id(), 1);
        assert_eq!(ConditionType::IconSet.get_id(), 6);

        // Test for_id
        assert_eq!(ConditionType::for_id(1), Some(ConditionType::CellValueIs));
        assert_eq!(ConditionType::for_id(6), Some(ConditionType::IconSet));
        assert_eq!(ConditionType::for_id(7), None);

        // Test TryFrom
        assert_eq!(ConditionType::try_from(2), Ok(ConditionType::Formula));
        assert!(ConditionType::try_from(0).is_err());

        // Test From
        let id: u8 = ConditionType::ColorScale.into();
        assert_eq!(id, 3);

        // Test type name
        assert_eq!(ConditionType::CellValueIs.get_type_name(), Some("cellIs"));
        assert_eq!(ConditionType::Filter.get_type_name(), None);

        // Test for_type_name
        assert_eq!(
            ConditionType::for_type_name("dataBar"),
            Some(ConditionType::DataBar)
        );
        assert_eq!(ConditionType::for_type_name("invalid"), None);

        // Test utility methods
        assert!(ConditionType::CellValueIs.uses_formula());
        assert!(!ConditionType::ColorScale.uses_formula());

        assert!(ConditionType::DataBar.uses_visual_formatting());
        assert!(!ConditionType::Formula.uses_visual_formatting());

        assert!(ConditionType::Filter.is_filter());
        assert!(!ConditionType::IconSet.is_filter());

        assert!(ConditionType::IconSet.requires_thresholds());
        assert!(!ConditionType::Formula.requires_thresholds());

        // Test Display
        assert_eq!(ConditionType::Filter.to_string(), "5 - Filter");

        // Test registry
        let registry = ConditionTypeRegistry::global();
        assert_eq!(registry.for_id(4), Some(ConditionType::DataBar));
        assert_eq!(registry.all().len(), 6);

        // Test constants
        assert_eq!(ConditionType::CELL_VALUE_IS, ConditionType::CellValueIs);
        assert_eq!(ConditionType::DATA_BAR, ConditionType::DataBar);
    }
}
