use crate::core::poi::ss::formula::functions::free_ref_function::FreeRefFunction;

/// Common interface for "Add-in" libraries and user defined function libraries.
pub trait UDFFinder {
    /// Returns executor by specified name. Returns `None` if the function name is unknown.
    ///
    /// # Arguments
    /// * `name` - Name of function
    ///
    /// # Returns
    /// * Function executor or `None` if not found
    fn find_function(&self, name: &str) -> Option<Box<dyn FreeRefFunction>>;
}
