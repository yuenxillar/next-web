//! Shiro's `Nameable` contract.
//!
//! A component that carries a name the configuration of a chain can set once
//! the component is built — the filters and filter chains of an application are
//! named before they run, and a chain resolves a filter by that name.

use next_web_core::Named;

/// A component whose name is readable and settable.
///
/// The name is what a filter chain resolves a component by, so it is the
/// counterpart of [`Named`](next_web_core::Named) for the components that are
/// named at runtime instead of at compile time.
pub trait Nameable: Named {
    /// Sets the name of this component.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the component.
    fn set_name(&mut self, name: &str);
}
