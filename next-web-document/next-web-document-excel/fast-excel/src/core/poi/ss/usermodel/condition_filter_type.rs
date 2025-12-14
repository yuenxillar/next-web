/// Used primarily for XSSF conditions, which defines a multitude of additional "filter" types
/// for conditional formatting. HSSF rules will always be None (not a filter type) or Some(ConditionFilterType::Filter).
/// XSSF conditions will be None (not a filter type) or any value other than Some(ConditionFilterType::Filter).
///
/// Variant names match the constants from `STCfType` for convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ConditionFilterType {
    /// This is the only value valid for HSSF rules
    Filter,
    Top10,
    UniqueValues,
    DuplicateValues,
    ContainsText,
    NotContainsText,
    BeginsWith,
    EndsWith,
    ContainsBlanks,
    NotContainsBlanks,
    ContainsErrors,
    NotContainsErrors,
    TimePeriod,
    AboveAverage,
}
