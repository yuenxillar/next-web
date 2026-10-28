use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CycleInRoleHierarchyError;

impl Display for CycleInRoleHierarchyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cycle detected in role hierarchy")
    }
}

impl std::error::Error for CycleInRoleHierarchyError {}
