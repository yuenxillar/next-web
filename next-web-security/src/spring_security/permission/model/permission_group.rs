use std::collections::HashSet;


#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum CombinationMode {
    #[default]
    And,
    Or,
}

#[derive(Default, Clone)]
pub struct PermissionGroup {
    role_group: Option<Vec<String>>,
    permission_group: Option<Vec<String>>,
    mode: CombinationMode,
}

impl PermissionGroup {
    pub fn new(role_group: Option<Vec<String>>, permission_group: Option<Vec<String>>) -> Self {
        Self {
            role_group,
            permission_group,
            mode: CombinationMode::And,
        }
    }

    pub fn roles<T: ToString>(mut self, role_group: Vec<T>) -> Self {
        self.role_group = Some(role_group.iter().map(|v| v.to_string()).collect());
        self
    }

    pub fn permissions<T: ToString>(mut self, role_group: Vec<T>) -> Self {
        self.permission_group = Some(role_group.iter().map(|v| v.to_string()).collect());
        self
    }

    pub fn mode(mut self, mode: CombinationMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn get_roles(&self) -> Option<&Vec<String>> {
        self.role_group.as_ref()
    }

    pub fn get_permissions(&self) -> Option<&Vec<String>> {
        self.permission_group.as_ref()
    }

    pub fn check(&self) -> bool {
        self.role_group.is_some() && self.permission_group.is_some()
    }

    pub fn get_mode(&self) -> &CombinationMode {
        &self.mode
    }
    pub fn valid(&self) -> bool {
        self.role_group
            .as_ref()
            .map(|v| v.len() == 0)
            .unwrap_or_default()
            == self
                .permission_group
                .as_ref()
                .map(|v| v.len() == 0)
                .unwrap_or_default()
    }

    pub fn match_value<T: AsRef<str> + PartialEq + Eq + ToString>(
        &self,
        var1: Option<&Vec<T>>,
        var2: Option<&Vec<T>>,
    ) -> bool {
        // if user_role first element is *, return true
        let var10 = var2
            .map(|s| s.get(0).map(|s| s.as_ref() == "*").unwrap_or(false))
            .unwrap_or_default();
        if var10 {
            return true;
        }

        let var11 = var1.map(|s| s.len()).unwrap_or_default();
        let var12 = var2.map(|s| s.len()).unwrap_or_default();
        match self.mode {
            CombinationMode::And => {
                if var11 != var12 {
                    return false;
                }

                let var4 =
                    var1.map(|item| item.iter().map(|s| s.as_ref()).collect::<HashSet<&str>>());
                let var5 =
                    var2.map(|item| item.iter().map(|s| s.as_ref()).collect::<HashSet<&str>>());
                return var4 == var5;
            }
            CombinationMode::Or => {
      
                if (var11 == 0)  && (var12 == 0) {
                    return true;
                }

                return var1.map(|s| {
                    s.iter().any(|s| {
                        var2.map(|s2| s2.contains(s)).unwrap_or_default()
                    })
                }).unwrap_or_default()
            }
        }
    }
}