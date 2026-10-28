use std::{
    collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
    sync::Arc,
};

use tracing::debug;

use crate::{
    access::hierarchicalroles::{
        cycle_in_role_hierarchy_error::CycleInRoleHierarchyError, role_hierarchy::RoleHierarchy,
    },
    core::{authority::SimpleGrantedAuthority, GrantedAuthority},
};

/// This struct defines a role hierarchy for use with various access checking components.
#[derive(Clone, Default)]
pub struct RoleHierarchyImpl {
    roles_reachable_in_one_or_more_steps_map: BTreeMap<String, BTreeSet<Arc<dyn GrantedAuthority>>>,
}

impl RoleHierarchyImpl {
    pub fn new(hierarchy: BTreeMap<String, BTreeSet<Arc<dyn GrantedAuthority>>>) -> Self {
        Self::build_roles_reachable_in_one_or_more_steps_map(hierarchy).expect(
            "
            Failed to build role hierarchy from one-step map",
        )
    }

    /// Create a role hierarchy instance with the given definition, similar to the following:
    /// ROLE_A > ROLE_B
    /// ROLE_B > ROLE_AUTHENTICATED
    /// ROLE_AUTHENTICATED > ROLE_UNAUTHENTICATED
    pub fn from_hierarchy(hierarchy: impl AsRef<str>) -> Result<Self, CycleInRoleHierarchyError> {
        Self::build_roles_reachable_in_one_or_more_steps_map(
            Self::build_roles_reachable_in_one_step_map(hierarchy.as_ref()),
        )
    }

    /// Factory method that creates a RoleHierarchyImpl.Builder instance with the default role prefix "ROLE_"
    pub fn with_default_role_prefix() -> RoleHierarchyBuilder {
        Self::with_role_prefix("ROLE_")
    }

    /// Factory method that creates a RoleHierarchyImpl.Builder instance with the specified role prefix.
    pub fn with_role_prefix(role_prefix: impl Into<String>) -> RoleHierarchyBuilder {
        RoleHierarchyBuilder::new(role_prefix)
    }

    /// Parse input and build the map for the roles reachable in one step:
    /// the higher role will become a key that references a set of the reachable lower roles.
    fn build_roles_reachable_in_one_step_map(
        hierarchy: &str,
    ) -> BTreeMap<String, BTreeSet<Arc<dyn GrantedAuthority>>> {
        let mut roles_reachable_in_one_or_more_steps_map =
            BTreeMap::<String, BTreeSet<Arc<dyn GrantedAuthority>>>::new();
        for line in hierarchy
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            let roles = line
                .split('>')
                .map(str::trim)
                .filter(|role| !role.is_empty())
                .collect::<Vec<_>>();
            for pair in roles.windows(2) {
                let higher = pair[0].to_string();
                let lower = pair[1].to_string();
                debug!(
                               "build_roles_reachable_in_one_step_map - From role {} one can reach role {} in one step",
                               higher, lower
                           );
                roles_reachable_in_one_or_more_steps_map
                    .entry(higher)
                    .or_default()
                    .insert(Arc::new(SimpleGrantedAuthority::new(lower)));
            }
        }

        roles_reachable_in_one_or_more_steps_map
    }

    /// For every higher role from rolesReachableInOneStepMap store all roles that are reachable
    /// from it in the map of roles reachable in one or more steps.
    /// (Or throw a CycleInRoleHierarchyException if a cycle in the role hierarchy definition is detected)
    fn build_roles_reachable_in_one_or_more_steps_map(
        hierarchy: BTreeMap<String, BTreeSet<Arc<dyn GrantedAuthority>>>,
    ) -> Result<Self, CycleInRoleHierarchyError> {
        let mut roles_reachable_in_one_or_more_steps_map = BTreeMap::new();
        // iterate over all higher roles from rolesReachableInOneStepMap
        for role_name in hierarchy.keys() {
            let mut to_visit = hierarchy
                .get(role_name)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect::<VecDeque<_>>();
            let mut visited = BTreeSet::new();

            while let Some(lower_role) = to_visit.pop_front() {
                if lower_role.authority() == Some(role_name.as_str()) {
                    return Err(CycleInRoleHierarchyError);
                }
                if !visited.insert(lower_role.clone()) {
                    continue;
                }
                if let Some(children) = hierarchy.get(lower_role.authority().unwrap_or_default()) {
                    for child in children {
                        to_visit.push_back(child.clone());
                    }
                }
            }

            // debug!(
            //                "build_roles_reachable_in_one_or_more_steps_map - From role {} one can reach {:?} in one or more steps",
            //                role_name,
            //                &visited
            //            );
            roles_reachable_in_one_or_more_steps_map.insert(role_name.to_owned(), visited);
        }

        Ok(Self {
            roles_reachable_in_one_or_more_steps_map,
        })
    }
}

impl RoleHierarchy for RoleHierarchyImpl {
    fn reachable_granted_authorities(
        &self,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        if authorities.is_empty() {
            return Vec::new();
        }

        let mut reachable_roles = HashSet::new();
        let mut processed_names = HashSet::new();
        for authority in authorities {
            // Do not process authorities without string representation
            let auth_name = match authority.authority() {
                Some(name) => name,
                None => {
                    reachable_roles.insert(Arc::clone(authority));
                    continue;
                }
            };
            // Do not process already processed roles
            if !processed_names.insert(auth_name) {
                continue;
            }
            // Add original authority
            reachable_roles.insert(Arc::clone(authority));
            // Add roles reachable in one or more steps
            if let Some(lower_roles) = self.roles_reachable_in_one_or_more_steps_map.get(auth_name)
            {
                for role in lower_roles {
                    if let Some(authority) = role.authority() {
                        if processed_names.insert(authority) {
                            reachable_roles.insert(Arc::clone(role));
                        }
                    }
                }
            }
        }

        // let var1 = authorities
        //     .iter()
        //     .filter_map(|a| a.authority())
        //     .collect::<Vec<_>>();
        // let var2 = reachable_roles
        //     .iter()
        //     .filter_map(|r| r.authority())
        //     .collect::<Vec<_>>();
        // debug!(
        //            "get_reachable_granted_authorities - From the roles {:?} one can reach {:?} in zero or more steps",
        //            var1, var2);

        reachable_roles.into_iter().collect()
    }
}

/// Builder class for constructing a RoleHierarchyImpl based on a hierarchical role structure.
#[derive(Clone)]
pub struct RoleHierarchyBuilder {
    role_prefix: String,
    hierarchy: BTreeMap<String, BTreeSet<Arc<dyn GrantedAuthority>>>,
}

impl RoleHierarchyBuilder {
    pub fn new(role_prefix: impl Into<String>) -> Self {
        Self {
            role_prefix: role_prefix.into(),
            hierarchy: BTreeMap::new(),
        }
    }

    /// Creates a new hierarchy branch to define a role and its child roles.
    pub fn role<'a>(&'a mut self, role: impl Into<String>) -> ImpliedRoles<'a> {
        let role = role.into();
        assert!(!role.trim().is_empty(), "role must not be empty");
        ImpliedRoles {
            builder: self,
            role,
        }
    }

    /// Builds and returns a RoleHierarchyImpl describing the defined role hierarchy.
    pub fn build(self) -> Result<RoleHierarchyImpl, CycleInRoleHierarchyError> {
        RoleHierarchyImpl::build_roles_reachable_in_one_or_more_steps_map(self.hierarchy)
    }

    fn add_hierarchy(&mut self, role: String, implied_roles: Vec<String>) {
        assert!(
            !implied_roles.is_empty(),
            "at least one implied role must be provided"
        );
        let role = format!("{}{}", self.role_prefix, role);
        let implied = self.hierarchy.entry(role).or_default();
        for implied_role in implied_roles {
            assert!(
                !implied_role.trim().is_empty(),
                "implied role name cannot be empty"
            );
            implied.insert(Arc::new(SimpleGrantedAuthority::new(format!(
                "{}{}",
                self.role_prefix, implied_role
            ))));
        }
    }
}

/// Builder class for constructing child roles within a role hierarchy branch.
pub struct ImpliedRoles<'a> {
    builder: &'a mut RoleHierarchyBuilder,
    role: String,
}

impl<'a> ImpliedRoles<'a> {
    pub fn implies(
        self,
        implied_roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> &'a mut RoleHierarchyBuilder {
        self.builder.add_hierarchy(
            self.role,
            implied_roles.into_iter().map(Into::into).collect(),
        );
        self.builder
    }
}
