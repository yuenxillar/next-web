use hashbrown::HashSet;

#[derive(Default, Clone)]
pub struct AuthGroup {
    role_group: Option<Vec<String>>,
    permission_group: Option<Vec<String>>,
    combination: Option<CombinationGroup>,
    is_combination: bool,
}

impl AuthGroup {
    pub fn new(
        role_group: Option<Vec<String>>,
        permission_group: Option<Vec<String>>,
        combination: Option<CombinationGroup>,
    ) -> Self {
        Self {
            role_group,
            permission_group,
            combination,
            is_combination: false,
        }
    }

    pub fn roles(&self) -> Option<Vec<String>> {
        if self.is_combination {
            return self
                .combination
                .as_ref()
                .map(|v| v.role())
                .unwrap_or_default();
        }
        self.role_group.clone()
    }

    pub fn permissions(&self) -> Option<Vec<String>> {
        if self.is_combination {
            return self
                .combination
                .as_ref()
                .map(|v| v.permission())
                .unwrap_or_default();
        }
       self.permission_group.clone()
    }

    pub fn mode(&self) -> Option<CombinationMode> {
        if self.is_combination {
            return self
                .combination
                .as_ref()
                .map(|v| v.mode.clone());
        }
        None
    }

    pub fn combination(&self) -> Option<CombinationGroup> {
        self.combination.clone()
    }

    pub fn is_combination(&self) -> bool {
        self.combination.is_some()
    }

    pub fn combination_valid(&self) -> bool {
        if let Some(combination) = &self.combination {
            let var1 = combination.role_group.as_ref().map(|v| v.len() == 0)
                == combination
                    .permission_group
                    .as_ref()
                    .map(|v| v.len() == 0);
            return var1;
        }
        false
    }

    pub fn set_combination(&mut self, flag: bool) {
        self.is_combination = flag;
    }

    pub fn match_value(
        &self,
        var1: Vec<String>,
        var2: Vec<String>,
        mode: Option<&CombinationMode>,
    ) -> bool {
        let var3 = var2.len() == 1 && var2[0] == "*";
        if var3 {
            return true;
        }
        if self.is_combination {
            match mode {
                Some(CombinationMode::And) => {
                    if var1.len() != var2.len() {
                        return false;
                    }
                    let var4: HashSet<String> = var1.into_iter().collect();
                    let var5: HashSet<String> = var2.into_iter().collect();
                    return var4 == var5;
                }
                Some(CombinationMode::Or) => {
                    return var1.iter().any(|v| var2.contains(v));
                }
                None => {}
            }
        } else {
            if var1.len() != var2.len() {
                return false;
            }
            let var4: HashSet<String> = var1.into_iter().collect();
            let var5: HashSet<String> = var2.into_iter().collect();
            return var4 == var5;
        }
        false
    }
}

#[derive(Clone)]
pub struct CombinationGroup {
    role_group: Option<(Vec<String>)>,
    permission_group: Option<Vec<String>>,
    mode: CombinationMode
}

impl CombinationGroup {
    pub fn new(
        role_group: Option<Vec<String>>,
        permission_group: Option<Vec<String>>,
        mode: CombinationMode
    ) -> Self {
        Self {
            role_group,
            permission_group,
            mode,
        }
    }

    pub fn role_group(&self) -> Option<Vec<String>> {
        self.role_group.clone()
    }

    pub fn permission_group(&self) -> Option<Vec<String>> {
        self.permission_group.clone()
    }

    fn role(&self) -> Option<Vec<String>> {
        self.role_group()
            .map(|n| Some(n.clone()))
            .unwrap_or_default()
    }

    fn permission(&self) -> Option<Vec<String>> {
        self.permission_group()
            .map(|n| Some(n.clone()))
            .unwrap_or_default()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Deserialize)]
pub enum CombinationMode {
    And,
    Or,
}
