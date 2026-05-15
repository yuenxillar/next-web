pub struct RoleHierarchyUtils;

impl RoleHierarchyUtils {
    pub fn role_hierarchy_from_map(
        role_hierarchy_map: impl IntoIterator<Item = (impl Into<String>, Vec<String>)>,
    ) -> String {
        role_hierarchy_map
            .into_iter()
            .flat_map(|(higher, lowers)| {
                let higher = higher.into();
                lowers
                    .into_iter()
                    .map(move |lower| format!("{} > {}", higher, lower))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
