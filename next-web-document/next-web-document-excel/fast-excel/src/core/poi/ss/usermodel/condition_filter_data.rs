/// These values are needed by various conditional formatting evaluation filter types
pub trait ConditionFilterData {
    /// Returns true if the flag is missing or set to true
    fn get_above_average(&self) -> bool;

    /// Returns true if the flag is set
    fn get_bottom(&self) -> bool;

    /// Returns true if the flag is set
    fn get_equal_average(&self) -> bool;

    /// Returns true if the flag is set
    fn get_percent(&self) -> bool;

    /// Returns value, or 0 if not used/defined
    fn get_rank(&self) -> i64;

    /// Returns value, or 0 if not used/defined
    fn get_std_dev(&self) -> i32;
}
