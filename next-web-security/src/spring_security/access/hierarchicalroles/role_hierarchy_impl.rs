use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use crate::{
    access::hierarchicalroles::{
        cycle_in_role_hierarchy_error::CycleInRoleHierarchyError, role_hierarchy::RoleHierarchy,
    },
    core::granted_authority::GrantedAuthority,
};

#[derive(Clone, Debug, Default)]
pub struct RoleHierarchyImpl {
    reachable_roles: BTreeMap<String, BTreeSet<String>>,
}

impl RoleHierarchyImpl {
    pub fn from_hierarchy(hierarchy: impl AsRef<str>) -> Result<Self, CycleInRoleHierarchyError> {
        Self::from_one_step_map(Self::build_one_step_map(hierarchy.as_ref()))
    }

    pub fn with_default_role_prefix() -> RoleHierarchyBuilder {
        Self::with_role_prefix("ROLE_")
    }

    pub fn with_role_prefix(role_prefix: impl Into<String>) -> RoleHierarchyBuilder {
        RoleHierarchyBuilder::new(role_prefix)
    }

    pub fn from_one_step_map(
        hierarchy: BTreeMap<String, BTreeSet<String>>,
    ) -> Result<Self, CycleInRoleHierarchyError> {
        let mut reachable_roles = BTreeMap::new();

        for role_name in hierarchy.keys() {
            let mut to_visit = hierarchy
                .get(role_name)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect::<VecDeque<_>>();
            let mut visited = BTreeSet::new();

            while let Some(lower_role) = to_visit.pop_front() {
                if lower_role == *role_name {
                    return Err(CycleInRoleHierarchyError);
                }
                if !visited.insert(lower_role.clone()) {
                    continue;
                }
                if let Some(children) = hierarchy.get(&lower_role) {
                    for child in children {
                        to_visit.push_back(child.clone());
                    }
                }
            }

            reachable_roles.insert(role_name.clone(), visited);
        }

        Ok(Self { reachable_roles })
    }

    fn build_one_step_map(hierarchy: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut map = BTreeMap::<String, BTreeSet<String>>::new();
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
                map.entry(pair[0].to_string())
                    .or_default()
                    .insert(pair[1].to_string());
            }
        }
        map
    }
}

impl RoleHierarchy for RoleHierarchyImpl {
    fn reachable_granted_authorities(
        &self,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        let mut reachable = BTreeSet::new();
        for authority in authorities {
            reachable.insert(authority.clone());
            if let Some(lower_roles) = self.reachable_roles.get(authority) {
                reachable.extend(lower_roles.iter().cloned());
            }
        }
        reachable.into_iter().collect()
    }
}

#[derive(Clone, Debug)]
pub struct RoleHierarchyBuilder {
    role_prefix: String,
    hierarchy: BTreeMap<String, BTreeSet<String>>,
}

impl RoleHierarchyBuilder {
    pub fn new(role_prefix: impl Into<String>) -> Self {
        Self {
            role_prefix: role_prefix.into(),
            hierarchy: BTreeMap::new(),
        }
    }

    pub fn role<'a>(&'a mut self, role: impl Into<String>) -> ImpliedRoles<'a> {
        let role = role.into();
        assert!(!role.trim().is_empty(), "role must not be empty");
        ImpliedRoles {
            builder: self,
            role,
        }
    }

    pub fn build(self) -> Result<RoleHierarchyImpl, CycleInRoleHierarchyError> {
        RoleHierarchyImpl::from_one_step_map(self.hierarchy)
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
            implied.insert(format!("{}{}", self.role_prefix, implied_role));
        }
    }
}

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
