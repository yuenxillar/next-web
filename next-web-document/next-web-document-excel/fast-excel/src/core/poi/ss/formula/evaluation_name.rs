use crate::core::poi::ss::formula::ptg::{Ptg, name_ptg::NamePtg};

/// Abstracts a name record for formula evaluation.
pub trait EvaluationName: Send + Sync {
    /// Get the name text
    ///
    /// # Returns
    /// * Name text
    fn get_name_text(&self) -> &str;

    /// Check if this is a function name
    ///
    /// # Returns
    /// * `true` if this is a function name
    fn is_function_name(&self) -> bool;

    /// Check if this name has a formula definition
    ///
    /// # Returns
    /// * `true` if this name has a formula definition
    fn has_formula(&self) -> bool;

    /// Get the name definition as parse tree grammar tokens
    ///
    /// # Returns
    /// * Array of PTG tokens defining the name
    fn get_name_definition(&self) -> Vec<&Ptg>;

    /// Check if this name defines a range
    ///
    /// # Returns
    /// * `true` if this name defines a range
    fn is_range(&self) -> bool;

    /// Create a NamePtg for this name
    ///
    /// # Returns
    /// * NamePtg instance
    fn create_ptg(&self) -> NamePtg;
}
